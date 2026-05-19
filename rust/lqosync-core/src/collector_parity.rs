use crate::protocol::Diagnostic;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

const DEFAULT_COMPARE_FIELDS: &[&str] = &[
    "Parent Node",
    "MAC",
    "IPv4",
    "Download Min Mbps",
    "Upload Min Mbps",
    "Download Max Mbps",
    "Upload Max Mbps",
    "Comment",
];

fn sval(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn row_key(row: &Value) -> String {
    for key in ["Circuit Name", "Circuit ID", "Device ID", "Device Name"] {
        let value = sval(row, key);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

fn rows_from_value(value: &Value) -> Vec<Value> {
    if let Some(items) = value.as_array() {
        return items.iter().filter(|v| v.is_object()).cloned().collect();
    }
    if let Some(obj) = value.as_object() {
        // Accept Python's existing_data style: {circuit_code: row}
        return obj.values().filter(|v| v.is_object()).cloned().collect();
    }
    Vec::new()
}

fn payload_rows(payload: &Value, key: &str) -> Vec<Value> {
    payload.get(key).map(rows_from_value).unwrap_or_default()
}

fn rust_bundle_rows(payload: &Value) -> Vec<Value> {
    if !payload_rows(payload, "rust_rows").is_empty() {
        return payload_rows(payload, "rust_rows");
    }
    if !payload_rows(payload, "normalized_rows").is_empty() {
        return payload_rows(payload, "normalized_rows");
    }
    let bundle = payload.get("rust_bundle").unwrap_or(&Value::Null);
    if let Some(result) = bundle.get("result") {
        if let Some(rows) = result.get("normalized_rows") {
            return rows_from_value(rows);
        }
    }
    if let Some(rows) = bundle.get("normalized_rows") {
        return rows_from_value(rows);
    }
    Vec::new()
}

fn compare_fields(payload: &Value) -> Vec<String> {
    let fields = payload.get("compare_fields").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    if fields.is_empty() {
        DEFAULT_COMPARE_FIELDS.iter().map(|s| s.to_string()).collect()
    } else {
        fields
    }
}

fn index_rows(rows: &[Value], label: &str, warnings: &mut Vec<Diagnostic>) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for (idx, row) in rows.iter().enumerate() {
        let key = row_key(row);
        if key.is_empty() {
            warnings.push(Diagnostic::warning(
                "parity_row_missing_key",
                Some(format!("{label}[{idx}]")),
                format!("{label} row at index {idx} has no Circuit Name/Circuit ID/Device ID/Device Name and was skipped."),
            ));
            continue;
        }
        if !seen.insert(key.clone()) {
            warnings.push(Diagnostic::warning(
                "parity_duplicate_key",
                Some(format!("{label}.{key}")),
                format!("Duplicate circuit key {key} in {label}; the last row wins for parity comparison."),
            ));
        }
        out.insert(key, row.clone());
    }
    out
}

pub fn compare_collector_bundle_parity_payload(payload: &Value) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut errors: Vec<Diagnostic> = Vec::new();
    let python_rows = payload_rows(payload, "python_rows");
    let rust_rows = rust_bundle_rows(payload);
    let fields = compare_fields(payload);
    let strict = payload.get("strict").and_then(Value::as_bool).unwrap_or(false);
    let max_mismatches = payload.get("max_mismatches").and_then(Value::as_u64).unwrap_or(50) as usize;
    let warning_threshold = payload.get("warning_threshold_percent").and_then(Value::as_f64).unwrap_or(95.0);

    let py = index_rows(&python_rows, "python_rows", &mut warnings);
    let rs = index_rows(&rust_rows, "rust_rows", &mut warnings);
    let py_keys: BTreeSet<String> = py.keys().cloned().collect();
    let rs_keys: BTreeSet<String> = rs.keys().cloned().collect();
    let missing_in_rust: Vec<String> = py_keys.difference(&rs_keys).cloned().collect();
    let extra_in_rust: Vec<String> = rs_keys.difference(&py_keys).cloned().collect();
    let common: Vec<String> = py_keys.intersection(&rs_keys).cloned().collect();

    let mut field_mismatches: Vec<Value> = Vec::new();
    let mut field_mismatch_count = 0usize;
    for key in &common {
        let prow = py.get(key).unwrap_or(&Value::Null);
        let rrow = rs.get(key).unwrap_or(&Value::Null);
        for field in &fields {
            let pv = sval(prow, field);
            let rv = sval(rrow, field);
            if pv != rv {
                field_mismatch_count += 1;
                if field_mismatches.len() < max_mismatches {
                    field_mismatches.push(json!({"circuit": key, "field": field, "python": pv, "rust": rv}));
                }
            }
        }
    }

    let total_key_checks = py_keys.union(&rs_keys).count();
    let total_field_checks = common.len() * fields.len();
    let total_checks = total_key_checks + total_field_checks;
    let failed_checks = missing_in_rust.len() + extra_in_rust.len() + field_mismatch_count;
    let parity_score = if total_checks == 0 {
        100.0
    } else {
        ((total_checks.saturating_sub(failed_checks)) as f64 / total_checks as f64) * 100.0
    };
    let exact_match = failed_checks == 0;
    let verdict = if exact_match {
        "parity_pass"
    } else if parity_score >= warning_threshold {
        "parity_warning"
    } else {
        "parity_failed"
    };

    if !exact_match {
        warnings.push(Diagnostic::warning(
            "collector_bundle_parity_mismatch",
            Some("collector_bundle_parity".to_string()),
            format!("Rust collector bundle shadow output differs from Python authoritative rows: score={parity_score:.2}%, missing={}, extra={}, field_mismatches={field_mismatch_count}.", missing_in_rust.len(), extra_in_rust.len()),
        ));
    }
    if strict && verdict == "parity_failed" {
        errors.push(Diagnostic::error(
            "collector_bundle_parity_failed",
            Some("collector_bundle_parity".to_string()),
            "Strict collector bundle parity check failed. Python remains authoritative; do not enable collector authority yet.",
        ));
    }

    let result = json!({
        "mode": "collector_bundle_parity_shadow",
        "verdict": verdict,
        "exact_match": exact_match,
        "parity_score": (parity_score * 100.0).round() / 100.0,
        "python_count": py.len(),
        "rust_count": rs.len(),
        "matched_count": common.len(),
        "missing_in_rust_count": missing_in_rust.len(),
        "extra_in_rust_count": extra_in_rust.len(),
        "field_mismatch_count": field_mismatch_count,
        "mismatch_sample_count": field_mismatches.len(),
        "missing_in_rust": missing_in_rust,
        "extra_in_rust": extra_in_rust,
        "field_mismatches": field_mismatches,
        "compare_fields": fields,
        "strict": strict,
        "authority_note": "Python collector/builders remain authoritative. This parity report is diagnostic until a future collector authority pilot."
    });
    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_exact_parity() {
        let rows = json!([
            {"Circuit Name":"c1", "Parent Node":"P", "MAC":"AA", "IPv4":"10.0.0.1", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Comment":"PPP"}
        ]);
        let (result, errors, warnings) = compare_collector_bundle_parity_payload(&json!({"python_rows": rows, "rust_rows": rows}));
        assert!(errors.is_empty());
        assert!(warnings.is_empty());
        assert_eq!(result.get("verdict").and_then(Value::as_str), Some("parity_pass"));
        assert_eq!(result.get("exact_match").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn detects_field_and_key_mismatch() {
        let python_rows = json!([
            {"Circuit Name":"c1", "Parent Node":"P", "MAC":"AA", "IPv4":"10.0.0.1", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Comment":"PPP"},
            {"Circuit Name":"c2", "Parent Node":"P", "MAC":"BB", "IPv4":"10.0.0.2", "Download Max Mbps":"20", "Upload Max Mbps":"20", "Download Min Mbps":"10", "Upload Min Mbps":"10", "Comment":"DHCP"}
        ]);
        let rust_rows = json!([
            {"Circuit Name":"c1", "Parent Node":"P", "MAC":"AA", "IPv4":"10.0.0.99", "Download Max Mbps":"15", "Upload Max Mbps":"15", "Download Min Mbps":"7.5", "Upload Min Mbps":"7.5", "Comment":"PPP"},
            {"Circuit Name":"c3", "Parent Node":"P", "MAC":"CC", "IPv4":"10.0.0.3", "Download Max Mbps":"10", "Upload Max Mbps":"10", "Download Min Mbps":"5", "Upload Min Mbps":"5", "Comment":"HS"}
        ]);
        let (result, _errors, warnings) = compare_collector_bundle_parity_payload(&json!({"python_rows": python_rows, "rust_rows": rust_rows}));
        assert!(!warnings.is_empty());
        assert_eq!(result.get("missing_in_rust_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("extra_in_rust_count").and_then(Value::as_u64), Some(1));
        assert_eq!(result.get("field_mismatch_count").and_then(Value::as_u64), Some(1));
    }
}
