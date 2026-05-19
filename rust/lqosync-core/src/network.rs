use crate::protocol::Diagnostic;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub fn parse_network_text(text: &str) -> Result<Value, serde_json::Error> {
    let value: Value = serde_json::from_str(text)?;
    Ok(if value.is_object() { value } else { Value::Object(Default::default()) })
}

pub fn collect_node_names(network: &Value) -> HashSet<String> {
    let mut names = HashSet::new();
    if let Some(obj) = network.as_object() {
        for (name, node) in obj {
            collect_node(name, node, &mut names);
        }
    }
    names
}

pub fn validate_network(network: &Value) -> (Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut seen_paths: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(obj) = network.as_object() {
        for (name, node) in obj {
            validate_node(name, node, &mut vec![], &mut seen_paths, &mut errors, &mut warnings);
        }
    }
    for (name, paths) in seen_paths {
        if paths.len() > 1 {
            warnings.push(Diagnostic::warning(
                "duplicate_node_name",
                Some(format!("network.{name}")),
                format!("Node name '{name}' appears in multiple paths: {}", paths.join(", ")),
            ));
        }
    }
    (errors, warnings)
}

fn collect_node(name: &str, node: &Value, names: &mut HashSet<String>) {
    names.insert(name.to_string());
    if let Some(children) = node.get("children").and_then(|v| v.as_object()) {
        for (child_name, child) in children {
            collect_node(child_name, child, names);
        }
    }
}

fn validate_node(
    name: &str,
    node: &Value,
    path: &mut Vec<String>,
    seen_paths: &mut HashMap<String, Vec<String>>,
    errors: &mut Vec<Diagnostic>,
    warnings: &mut Vec<Diagnostic>,
) {
    path.push(name.to_string());
    let path_string = path.join("/");
    seen_paths.entry(name.to_string()).or_default().push(path_string.clone());

    for key in ["downloadBandwidthMbps", "uploadBandwidthMbps"] {
        if let Some(raw) = node.get(key) {
            if raw.as_f64().is_none() {
                errors.push(Diagnostic::error(
                    "invalid_node_bandwidth",
                    Some(format!("network.{path_string}.{key}")),
                    format!("Invalid node bandwidth {key} for {path_string}"),
                )
                .with_value(raw.clone()));
            }
        }
    }

    if let Some(children) = node.get("children") {
        if let Some(obj) = children.as_object() {
            for (child_name, child) in obj {
                validate_node(child_name, child, path, seen_paths, errors, warnings);
            }
        } else {
            errors.push(Diagnostic::error(
                "invalid_children",
                Some(format!("network.{path_string}.children")),
                format!("children must be an object for node {path_string}"),
            ));
        }
    }

    path.pop();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_names_and_flags_bad_bandwidth() {
        let network = parse_network_text(r#"{"Root":{"downloadBandwidthMbps":"bad","uploadBandwidthMbps":10,"type":"site","children":{"Child":{"downloadBandwidthMbps":1,"uploadBandwidthMbps":1,"children":{}}}}}"#).unwrap();
        let names = collect_node_names(&network);
        assert!(names.contains("Root"));
        assert!(names.contains("Child"));
        let (errors, _warnings) = validate_network(&network);
        assert!(errors.iter().any(|e| e.code == "invalid_node_bandwidth"));
    }
}
