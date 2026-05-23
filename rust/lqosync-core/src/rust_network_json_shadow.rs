use crate::network::{collect_node_names, parse_network_text, validate_network};
use crate::protocol::Diagnostic;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

#[derive(Clone, Debug)]
struct ShadowRow {
    parent_node: String,
    download_max_mbps: f64,
    upload_max_mbps: f64,
}

#[derive(Clone, Debug)]
struct ShadowMeta {
    circuit_name: String,
    source_type: String,
    router: String,
    server: String,
    profile: String,
}

fn sval(value: &Value, key: &str) -> String {
    match value.get(key) {
        Some(Value::String(s)) => s.trim().to_string(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn boolish(value: &Value, key: &str) -> bool {
    match value.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => matches!(
            s.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Some(Value::Number(n)) => n.as_i64().unwrap_or(0) != 0,
        _ => false,
    }
}

fn f64ish(value: &Value, key: &str) -> Option<f64> {
    match value.get(key) {
        Some(Value::Number(n)) => n.as_f64().filter(|v| v.is_finite()),
        Some(Value::String(s)) => s.trim().parse::<f64>().ok().filter(|v| v.is_finite()),
        _ => None,
    }
}

fn safe_mbps(value: f64, minimum: f64) -> f64 {
    let finite = if value.is_finite() { value } else { minimum };
    let rounded = (finite * 1000.0).round() / 1000.0;
    rounded.max(minimum)
}

fn make_node(download: f64, upload: f64, node_type: &str, virtual_flag: Option<bool>) -> Value {
    let mut map = Map::new();
    map.insert(
        "downloadBandwidthMbps".to_string(),
        json!(safe_mbps(download, 0.128)),
    );
    map.insert(
        "uploadBandwidthMbps".to_string(),
        json!(safe_mbps(upload, 0.128)),
    );
    map.insert("type".to_string(), json!(node_type));
    map.insert("children".to_string(), Value::Object(Map::new()));
    if let Some(flag) = virtual_flag {
        map.insert("virtual".to_string(), json!(flag));
    }
    Value::Object(map)
}

fn sort_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<String> = map.keys().cloned().collect();
            keys.sort();
            let mut ordered = Map::new();
            for key in keys {
                if let Some(child) = map.get(&key) {
                    ordered.insert(key, sort_value(child));
                }
            }
            Value::Object(ordered)
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_value).collect()),
        _ => value.clone(),
    }
}

fn render_network_text(value: &Value) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(&sort_value(value))?;
    text.push('\n');
    Ok(text)
}

fn get_network_mode(config: &Value) -> String {
    let mode = config
        .get("network_mode")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    match mode {
        "router_children" | "flat_router_root" | "flat_no_parent" | "deep_hierarchy"
        | "custom_hierarchy" => mode.to_string(),
        _ => {
            let flat = boolish(config, "flat_network");
            let no_parent = boolish(config, "no_parent");
            if flat && no_parent {
                "flat_no_parent".to_string()
            } else if flat {
                "flat_router_root".to_string()
            } else {
                "router_children".to_string()
            }
        }
    }
}

fn is_deep_hierarchy_mode(mode: &str) -> bool {
    matches!(mode, "deep_hierarchy" | "custom_hierarchy")
}

fn flat_parent_for_mode(mode: &str, router_name: &str) -> Option<String> {
    match mode {
        "flat_no_parent" => Some(String::new()),
        "flat_router_root" => Some(router_name.to_string()),
        _ => None,
    }
}

fn replace_template(
    template: &str,
    router: &str,
    profile: &str,
    server: &str,
    plan: &str,
) -> String {
    template
        .replace("{router}", router)
        .replace("{profile}", profile)
        .replace("{server}", server)
        .replace("{plan}", plan)
}

fn ppp_flat_node_name(router: &Value) -> String {
    let router_name = sval(router, "name");
    let template = router
        .get("pppoe")
        .and_then(|value| value.get("flat_node_name"))
        .and_then(Value::as_str)
        .unwrap_or("PPP-{router}");
    replace_template(template, &router_name, "", "", "")
}

fn dhcp_node_name(router: &Value, server: &Value, plan_label: Option<&str>) -> String {
    let router_name = sval(router, "name");
    let server_name = sval(server, "name");
    let default_tpl = if plan_label.is_some() {
        "PLAN-DHCP-{plan}-{router}"
    } else {
        "DHCP-{server}-{router}"
    };
    let template = server
        .get("node_name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(default_tpl);
    replace_template(
        template,
        &router_name,
        "",
        &server_name,
        plan_label.unwrap_or(""),
    )
}

fn hotspot_node_name(router: &Value) -> String {
    let router_name = sval(router, "name");
    let template = router
        .get("hotspot")
        .and_then(|value| value.get("node_name"))
        .and_then(Value::as_str)
        .unwrap_or("HS-{router}");
    replace_template(template, &router_name, "", "", "")
}

fn factor_for_profile(profile_name: &str, rules: &[Value]) -> (f64, f64) {
    let speed = profile_name.trim().chars().collect::<String>();
    let parsed_speed = {
        let trimmed = speed.trim();
        let digits: String = trimmed
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit() || *ch == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        if digits.is_empty() {
            None
        } else {
            let unit = digits
                .chars()
                .last()
                .filter(|ch| ch.is_ascii_alphabetic())
                .map(|ch| ch.to_ascii_lowercase());
            let number_text = if unit.is_some() {
                digits[..digits.len().saturating_sub(1)].trim().to_string()
            } else {
                digits.trim().to_string()
            };
            number_text.parse::<f64>().ok().map(|number| match unit {
                Some('k') => number * 0.001,
                Some('g') => number * 1000.0,
                _ => number,
            })
        }
    };
    let Some(speed_mbps) = parsed_speed.filter(|value| value.is_finite() && *value > 0.0) else {
        return (1.0, 1.0);
    };

    let mut ordered_rules: Vec<&Value> = rules.iter().collect();
    ordered_rules.sort_by(|lhs, rhs| {
        let lhs_max = f64ish(lhs, "max_plan_mbps").unwrap_or(999999.0);
        let rhs_max = f64ish(rhs, "max_plan_mbps").unwrap_or(999999.0);
        lhs_max
            .partial_cmp(&rhs_max)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for rule in ordered_rules {
        let max_plan = f64ish(rule, "max_plan_mbps").unwrap_or(999999.0);
        if speed_mbps <= max_plan {
            return (
                f64ish(rule, "download_factor").unwrap_or(1.0),
                f64ish(rule, "upload_factor").unwrap_or(1.0),
            );
        }
    }
    (1.0, 1.0)
}

fn remove_child(node: &mut Value, node_name: &str) -> Option<Value> {
    let children = node.get_mut("children")?.as_object_mut()?;
    if children.contains_key(node_name) {
        return children.remove(node_name);
    }
    for child in children.values_mut() {
        if let Some(removed) = remove_child(child, node_name) {
            return Some(removed);
        }
    }
    None
}

fn remove_node(network: &mut Map<String, Value>, node_name: &str) -> Option<Value> {
    if network.contains_key(node_name) {
        return network.remove(node_name);
    }
    for child in network.values_mut() {
        if let Some(removed) = remove_child(child, node_name) {
            return Some(removed);
        }
    }
    None
}

fn find_child_mut<'a>(node: &'a mut Value, node_name: &str) -> Option<&'a mut Value> {
    let children = node.get_mut("children")?.as_object_mut()?;
    find_node_mut(children, node_name)
}

fn find_node_mut<'a>(
    network: &'a mut Map<String, Value>,
    node_name: &str,
) -> Option<&'a mut Value> {
    if network.contains_key(node_name) {
        return network.get_mut(node_name);
    }
    for child in network.values_mut() {
        if let Some(found) = find_child_mut(child, node_name) {
            return Some(found);
        }
    }
    None
}

fn ensure_children(node: &mut Value) -> &mut Map<String, Value> {
    if !node.is_object() {
        *node = Value::Object(Map::new());
    }
    let map = node.as_object_mut().expect("node object");
    let needs_children = map.get("children").and_then(Value::as_object).is_none();
    if needs_children {
        map.insert("children".to_string(), Value::Object(Map::new()));
    }
    map.get_mut("children")
        .and_then(Value::as_object_mut)
        .expect("children object")
}

fn ensure_parent_node<'a>(network: &'a mut Map<String, Value>, parent_name: &str) -> &'a mut Value {
    if find_node_mut(network, parent_name).is_none() {
        network.insert(
            parent_name.to_string(),
            make_node(0.0, 0.0, "site", Some(true)),
        );
    }
    let node = find_node_mut(network, parent_name).expect("parent node exists");
    ensure_children(node);
    node
}

fn configure_router_node(node: &mut Value, router: &Value) {
    if !node.is_object() {
        *node = Value::Object(Map::new());
    }
    let map = node.as_object_mut().expect("router node object");
    map.insert(
        "downloadBandwidthMbps".to_string(),
        json!(f64ish(router, "root_download_mbps").unwrap_or(115.0)),
    );
    map.insert(
        "uploadBandwidthMbps".to_string(),
        json!(f64ish(router, "root_upload_mbps").unwrap_or(115.0)),
    );
    map.insert(
        "type".to_string(),
        json!(router
            .get("root_type")
            .and_then(Value::as_str)
            .unwrap_or("site")),
    );
    map.insert(
        "virtual".to_string(),
        json!(boolish(router, "root_virtual")),
    );
    if !map.get("children").and_then(Value::as_object).is_some() {
        map.insert("children".to_string(), Value::Object(Map::new()));
    }
}

fn ensure_router_node<'a>(
    network: &'a mut Map<String, Value>,
    router: &Value,
    allow_parent: bool,
) -> &'a mut Value {
    let router_name = sval(router, "name");
    let parent_name = sval(router, "parent_node");
    let mut node = remove_node(network, &router_name).unwrap_or_else(|| Value::Object(Map::new()));
    configure_router_node(&mut node, router);
    if allow_parent && !parent_name.is_empty() && parent_name != router_name {
        let parent = ensure_parent_node(network, &parent_name);
        let children = ensure_children(parent);
        children.insert(router_name.clone(), node);
    } else {
        network.insert(router_name.clone(), node);
    }
    let router_node = find_node_mut(network, &router_name).expect("router node exists");
    ensure_children(router_node);
    router_node
}

fn load_current_network(
    payload: &Value,
    warnings: &mut Vec<Diagnostic>,
) -> (Value, String, String) {
    if let Some(network) = payload
        .get("current_network")
        .filter(|value| value.is_object())
    {
        let text = render_network_text(network).unwrap_or_else(|_| "{}\n".to_string());
        return (network.clone(), "payload_network".to_string(), text);
    }

    if let Some(text) = payload.get("current_network_text").and_then(Value::as_str) {
        return match parse_network_text(text) {
            Ok(network) => (network, "payload_text".to_string(), text.to_string()),
            Err(err) => {
                warnings.push(Diagnostic::warning(
                    "rust_network_shadow_current_network_parse_failed",
                    Some("current_network_text".to_string()),
                    format!("Rust network shadow builder could not parse current network.json text: {err}"),
                ));
                (json!({}), "empty_fallback".to_string(), "{}\n".to_string())
            }
        };
    }

    let path = payload
        .get("current_network_path")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .get("config")
                .and_then(|value| value.get("paths"))
                .and_then(|value| value.get("network_json"))
                .and_then(Value::as_str)
        })
        .unwrap_or("");

    if path.is_empty() {
        return (json!({}), "empty_fallback".to_string(), "{}\n".to_string());
    }

    match fs::read_to_string(path) {
        Ok(text) => match parse_network_text(&text) {
            Ok(network) => (network, "file".to_string(), text),
            Err(err) => {
                warnings.push(Diagnostic::warning(
                    "rust_network_shadow_current_network_parse_failed",
                    Some("config.paths.network_json".to_string()),
                    format!("Rust network shadow builder could not parse current network.json at {path}: {err}"),
                ));
                (json!({}), "empty_fallback".to_string(), "{}\n".to_string())
            }
        },
        Err(err) => {
            warnings.push(Diagnostic::warning(
                "rust_network_shadow_current_network_read_failed",
                Some("config.paths.network_json".to_string()),
                format!("Rust network shadow builder could not read current network.json at {path}: {err}"),
            ));
            (json!({}), "empty_fallback".to_string(), "{}\n".to_string())
        }
    }
}

fn parse_shadow_rows(bundle: &Value) -> BTreeMap<String, ShadowRow> {
    let mut rows = BTreeMap::new();
    if let Some(items) = bundle.get("normalized_rows").and_then(Value::as_array) {
        for item in items {
            let circuit_name = sval(item, "Circuit Name");
            if circuit_name.is_empty() {
                continue;
            }
            rows.insert(
                circuit_name,
                ShadowRow {
                    parent_node: sval(item, "Parent Node"),
                    download_max_mbps: f64ish(item, "Download Max Mbps").unwrap_or(0.0),
                    upload_max_mbps: f64ish(item, "Upload Max Mbps").unwrap_or(0.0),
                },
            );
        }
    }
    rows
}

fn parse_shadow_meta(bundle: &Value) -> Vec<ShadowMeta> {
    bundle
        .get("meta")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|value| value.is_object())
                .map(|value| ShadowMeta {
                    circuit_name: sval(value, "circuit_name"),
                    source_type: {
                        let source_type = sval(value, "source_type");
                        if source_type.is_empty() {
                            sval(value, "source")
                        } else {
                            source_type
                        }
                    },
                    router: sval(value, "router"),
                    server: sval(value, "server"),
                    profile: sval(value, "profile"),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn starts_with_any(value: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| value.starts_with(prefix))
}

fn router_children_mut<'a>(
    network: &'a mut Map<String, Value>,
    router_name: &str,
) -> Option<&'a mut Map<String, Value>> {
    let router_node = find_node_mut(network, router_name)?;
    Some(ensure_children(router_node))
}

fn sum_rows<'a, I>(rows: I) -> (f64, f64, usize)
where
    I: Iterator<Item = &'a ShadowRow>,
{
    let mut download = 0.0;
    let mut upload = 0.0;
    let mut count = 0usize;
    for row in rows {
        download += row.download_max_mbps;
        upload += row.upload_max_mbps;
        count += 1;
    }
    (download, upload, count)
}

fn apply_pppoe_nodes(
    network: &mut Map<String, Value>,
    router: &Value,
    mode: &str,
    rows_by_circuit: &BTreeMap<String, ShadowRow>,
    meta: &[ShadowMeta],
    node_math: &mut Map<String, Value>,
) {
    if !router
        .get("pppoe")
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let router_name = sval(router, "name");
    let Some(flat_parent) = flat_parent_for_mode(mode, &router_name) else {
        if router_children_mut(network, &router_name).is_none() {
            return;
        }
        let router_pppoe = router.get("pppoe").unwrap_or(&Value::Null);
        let per_plan = boolish(router_pppoe, "per_plan_node");
        let children_names: Vec<String> = router_children_mut(network, &router_name)
            .map(|children| children.keys().cloned().collect())
            .unwrap_or_default();

        if per_plan {
            let generic = ppp_flat_node_name(router);
            if let Some(children) = router_children_mut(network, &router_name) {
                children.remove(&generic);
            }
            let mut desired = BTreeSet::new();
            let mut totals: BTreeMap<String, (f64, f64, usize, String)> = BTreeMap::new();
            for entry in meta
                .iter()
                .filter(|entry| entry.source_type == "PPP" && entry.router == router_name)
            {
                let Some(row) = rows_by_circuit.get(&entry.circuit_name) else {
                    continue;
                };
                let node_name = row.parent_node.trim().to_string();
                if node_name.is_empty() {
                    continue;
                }
                desired.insert(node_name.clone());
                let profile = if entry.profile.is_empty() {
                    "default".to_string()
                } else {
                    entry.profile.clone()
                };
                let bucket = totals
                    .entry(node_name.clone())
                    .or_insert((0.0, 0.0, 0usize, profile));
                bucket.0 += row.download_max_mbps;
                bucket.1 += row.upload_max_mbps;
                bucket.2 += 1;
            }
            if let Some(children) = router_children_mut(network, &router_name) {
                let stale: Vec<String> = children_names
                    .iter()
                    .filter(|name| {
                        name.ends_with(&format!("-{router_name}"))
                            && !desired.contains(*name)
                            && !starts_with_any(name, &["DHCP-", "HS-", "PLAN-DHCP-"])
                    })
                    .cloned()
                    .collect();
                for name in stale {
                    children.remove(&name);
                }
                let rules = router_pppoe
                    .get("factor_rules")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for (node_name, (raw_down, raw_up, count, profile)) in totals {
                    let (df, uf) = factor_for_profile(&profile, &rules);
                    let final_down = raw_down * df;
                    let final_up = raw_up * uf;
                    let new_node = make_node(
                        final_down,
                        final_up,
                        router_pppoe
                            .get("node_type")
                            .and_then(Value::as_str)
                            .unwrap_or("plan"),
                        None,
                    );
                    node_math.insert(
                        node_name.clone(),
                        json!({
                            "source": "PPPoE",
                            "mode": "per_plan",
                            "profile": profile,
                            "active_count": count,
                            "raw_download_mbps": safe_mbps(raw_down, 0.0),
                            "raw_upload_mbps": safe_mbps(raw_up, 0.0),
                            "download_factor": df,
                            "upload_factor": uf,
                            "final_download_mbps": new_node.get("downloadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "final_upload_mbps": new_node.get("uploadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "formula": format!("{count} active users, sum {} Mbps × {df} = {} Mbps", safe_mbps(raw_down, 0.0), new_node.get("downloadBandwidthMbps").and_then(Value::as_f64).unwrap_or(0.0)),
                        }),
                    );
                    if children.get(&node_name) != Some(&new_node) {
                        children.insert(node_name, new_node);
                    }
                }
            }
        } else {
            let (raw_down, raw_up, count) = sum_rows(
                meta.iter()
                    .filter(|entry| entry.source_type == "PPP" && entry.router == router_name)
                    .filter_map(|entry| rows_by_circuit.get(&entry.circuit_name)),
            );
            let factor = f64ish(router_pppoe, "flat_aggregate_factor").unwrap_or(0.3);
            let capped_down = raw_down.min(f64ish(router, "root_download_mbps").unwrap_or(115.0));
            let capped_up = raw_up.min(f64ish(router, "root_upload_mbps").unwrap_or(115.0));
            let node_name = ppp_flat_node_name(router);
            let new_node = make_node(
                capped_down * factor,
                capped_up * factor,
                router_pppoe
                    .get("node_type")
                    .and_then(Value::as_str)
                    .unwrap_or("plan"),
                None,
            );
            node_math.insert(
                node_name.clone(),
                json!({
                    "source": "PPPoE",
                    "mode": "flat",
                    "active_count": count,
                    "raw_download_mbps": safe_mbps(raw_down, 0.0),
                    "raw_upload_mbps": safe_mbps(raw_up, 0.0),
                    "download_factor": factor,
                    "upload_factor": factor,
                    "final_download_mbps": new_node.get("downloadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                    "final_upload_mbps": new_node.get("uploadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                    "formula": format!("sum {} Mbps × {factor}, capped by root = {} Mbps", safe_mbps(raw_down, 0.0), new_node.get("downloadBandwidthMbps").and_then(Value::as_f64).unwrap_or(0.0)),
                }),
            );
            if let Some(children) = router_children_mut(network, &router_name) {
                let stale: Vec<String> = children_names
                    .iter()
                    .filter(|name| {
                        name.ends_with(&format!("-{router_name}"))
                            && !starts_with_any(name, &["DHCP-", "PPP-", "HS-", "PLAN-DHCP-"])
                    })
                    .cloned()
                    .collect();
                for name in stale {
                    children.remove(&name);
                }
                if children.get(&node_name) != Some(&new_node) {
                    children.insert(node_name, new_node);
                }
            }
        }
        return;
    };

    let source_count = meta
        .iter()
        .filter(|entry| entry.source_type == "PPP" && entry.router == router_name)
        .count();
    node_math.insert(
        format!("PPP-flat-{router_name}"),
        json!({
            "source": "PPPoE",
            "mode": mode,
            "active_count": source_count,
            "formula": if flat_parent.is_empty() {
                "flat mode: PPPoE circuits use blank Parent Node"
            } else {
                "flat mode: PPPoE circuits point directly to router root"
            }
        }),
    );
}

fn apply_dhcp_nodes(
    network: &mut Map<String, Value>,
    router: &Value,
    mode: &str,
    rows_by_circuit: &BTreeMap<String, ShadowRow>,
    meta: &[ShadowMeta],
    node_math: &mut Map<String, Value>,
) {
    let dhcp = router.get("dhcp").unwrap_or(&Value::Null);
    if !dhcp
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let router_name = sval(router, "name");
    let Some(flat_parent) = flat_parent_for_mode(mode, &router_name) else {
        if router_children_mut(network, &router_name).is_none() {
            return;
        }
        let servers = dhcp
            .get("servers")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let child_names: Vec<String> = router_children_mut(network, &router_name)
            .map(|children| children.keys().cloned().collect())
            .unwrap_or_default();
        let mut desired = BTreeSet::new();

        for server in servers {
            if server
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(true)
                == false
            {
                continue;
            }
            let server_name = sval(&server, "name");
            if server_name.is_empty() {
                continue;
            }
            let source_rows: Vec<&ShadowRow> = meta
                .iter()
                .filter(|entry| {
                    entry.source_type == "DHCP"
                        && entry.router == router_name
                        && entry.server == server_name
                })
                .filter_map(|entry| rows_by_circuit.get(&entry.circuit_name))
                .collect();
            let mode_name = sval(&server, "mode").to_ascii_lowercase();
            let mode_name = if mode_name.is_empty() {
                "per_site".to_string()
            } else {
                mode_name
            };
            let download_factor = f64ish(&server, "download_factor").unwrap_or(0.5);
            let upload_factor = f64ish(&server, "upload_factor").unwrap_or(0.5);

            if mode_name == "per_plan" {
                let first = source_rows.first().copied();
                let per_down = first
                    .map(|row| row.download_max_mbps)
                    .or_else(|| f64ish(&server, "default_plan_down_mbps"))
                    .or_else(|| f64ish(&server, "download_limit_mbps"))
                    .unwrap_or(15.0);
                let per_up = first
                    .map(|row| row.upload_max_mbps)
                    .or_else(|| f64ish(&server, "default_plan_up_mbps"))
                    .or_else(|| f64ish(&server, "upload_limit_mbps"))
                    .unwrap_or(15.0);
                let plan_label = if (per_down - per_up).abs() < f64::EPSILON {
                    format!("{}M", per_down.trunc() as i64)
                } else {
                    format!("{}M-{}M", per_down.trunc() as i64, per_up.trunc() as i64)
                };
                let node_name = dhcp_node_name(router, &server, Some(&plan_label));
                desired.insert(node_name.clone());
                let (raw_down, raw_up, count) = sum_rows(source_rows.iter().copied());
                if count > 0 {
                    let new_node = make_node(
                        raw_down * download_factor,
                        raw_up * upload_factor,
                        "plan",
                        None,
                    );
                    node_math.insert(
                        node_name.clone(),
                        json!({
                            "source": "DHCP",
                            "mode": "per_plan",
                            "active_count": count,
                            "raw_download_mbps": safe_mbps(raw_down, 0.0),
                            "raw_upload_mbps": safe_mbps(raw_up, 0.0),
                            "download_factor": download_factor,
                            "upload_factor": upload_factor,
                            "final_download_mbps": new_node.get("downloadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "final_upload_mbps": new_node.get("uploadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "formula": format!("sum {} Mbps × {download_factor} = {} Mbps", safe_mbps(raw_down, 0.0), new_node.get("downloadBandwidthMbps").and_then(Value::as_f64).unwrap_or(0.0)),
                        }),
                    );
                    if let Some(children) = router_children_mut(network, &router_name) {
                        if children.get(&node_name) != Some(&new_node) {
                            children.insert(node_name.clone(), new_node);
                        }
                    }
                }
            } else {
                let node_name = dhcp_node_name(router, &server, None);
                desired.insert(node_name.clone());
                let (raw_down, raw_up, count) = sum_rows(source_rows.iter().copied());
                if count > 0 {
                    let new_node = make_node(
                        raw_down * download_factor,
                        raw_up * upload_factor,
                        server
                            .get("node_type")
                            .and_then(Value::as_str)
                            .unwrap_or("site"),
                        None,
                    );
                    let per_down = source_rows
                        .first()
                        .map(|row| row.download_max_mbps)
                        .unwrap_or(0.0);
                    let per_up = source_rows
                        .first()
                        .map(|row| row.upload_max_mbps)
                        .unwrap_or(0.0);
                    node_math.insert(
                        node_name.clone(),
                        json!({
                            "source": "DHCP",
                            "mode": "per_site",
                            "server": server_name,
                            "active_count": count,
                            "per_client_download_mbps": safe_mbps(per_down, 0.0),
                            "per_client_upload_mbps": safe_mbps(per_up, 0.0),
                            "download_factor": download_factor,
                            "upload_factor": upload_factor,
                            "final_download_mbps": new_node.get("downloadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "final_upload_mbps": new_node.get("uploadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                            "formula": format!("{count} leases × {} Mbps × {download_factor} = {} Mbps", safe_mbps(per_down, 0.0), new_node.get("downloadBandwidthMbps").and_then(Value::as_f64).unwrap_or(0.0)),
                        }),
                    );
                    if let Some(children) = router_children_mut(network, &router_name) {
                        if children.get(&node_name) != Some(&new_node) {
                            children.insert(node_name.clone(), new_node);
                        }
                    }
                } else if let Some(children) = router_children_mut(network, &router_name) {
                    children.remove(&node_name);
                }
            }
        }

        if let Some(children) = router_children_mut(network, &router_name) {
            let stale: Vec<String> = child_names
                .iter()
                .filter(|name| {
                    (name.starts_with("DHCP-") || name.starts_with("PLAN-DHCP-"))
                        && name.ends_with(&format!("-{router_name}"))
                        && !desired.contains(*name)
                })
                .cloned()
                .collect();
            for name in stale {
                children.remove(&name);
            }
        }
        return;
    };

    let active_count = meta
        .iter()
        .filter(|entry| entry.source_type == "DHCP" && entry.router == router_name)
        .count();
    node_math.insert(
        format!("DHCP-flat-{router_name}"),
        json!({
            "source": "DHCP",
            "mode": mode,
            "active_count": active_count,
            "formula": if flat_parent.is_empty() {
                "flat mode: DHCP circuits use blank Parent Node"
            } else {
                "flat mode: DHCP circuits point directly to router root"
            }
        }),
    );
}

fn apply_hotspot_nodes(
    network: &mut Map<String, Value>,
    router: &Value,
    mode: &str,
    rows_by_circuit: &BTreeMap<String, ShadowRow>,
    meta: &[ShadowMeta],
    node_math: &mut Map<String, Value>,
) {
    let hotspot = router.get("hotspot").unwrap_or(&Value::Null);
    if !hotspot
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let router_name = sval(router, "name");
    let source_rows: Vec<&ShadowRow> = meta
        .iter()
        .filter(|entry| entry.source_type == "HS" && entry.router == router_name)
        .filter_map(|entry| rows_by_circuit.get(&entry.circuit_name))
        .collect();

    let Some(flat_parent) = flat_parent_for_mode(mode, &router_name) else {
        let node_name = hotspot_node_name(router);
        let (raw_down, raw_up, count) = sum_rows(source_rows.iter().copied());
        if count > 0 {
            let new_node = make_node(
                raw_down * f64ish(hotspot, "download_factor").unwrap_or(1.0),
                raw_up * f64ish(hotspot, "upload_factor").unwrap_or(1.0),
                hotspot
                    .get("node_type")
                    .and_then(Value::as_str)
                    .unwrap_or("site"),
                None,
            );
            node_math.insert(
                node_name.clone(),
                json!({
                    "source": "Hotspot",
                    "mode": "per_client",
                    "active_count": count,
                    "raw_download_mbps": safe_mbps(raw_down, 0.0),
                    "raw_upload_mbps": safe_mbps(raw_up, 0.0),
                    "download_factor": f64ish(hotspot, "download_factor").unwrap_or(1.0),
                    "upload_factor": f64ish(hotspot, "upload_factor").unwrap_or(1.0),
                    "final_download_mbps": new_node.get("downloadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                    "final_upload_mbps": new_node.get("uploadBandwidthMbps").cloned().unwrap_or_else(|| json!(0.0)),
                    "formula": format!("sum {} Mbps × {} = {} Mbps", safe_mbps(raw_down, 0.0), f64ish(hotspot, "download_factor").unwrap_or(1.0), new_node.get("downloadBandwidthMbps").and_then(Value::as_f64).unwrap_or(0.0)),
                }),
            );
            if let Some(children) = router_children_mut(network, &router_name) {
                if children.get(&node_name) != Some(&new_node) {
                    children.insert(node_name, new_node);
                }
            }
        } else if let Some(children) = router_children_mut(network, &router_name) {
            children.remove(&node_name);
        }
        return;
    };

    if !source_rows.is_empty() {
        node_math.insert(
            format!("HS-flat-{router_name}"),
            json!({
                "source": "Hotspot",
                "mode": mode,
                "active_count": source_rows.len(),
                "formula": if flat_parent.is_empty() {
                    "flat mode: Hotspot circuits use blank Parent Node"
                } else {
                    "flat mode: Hotspot circuits point directly to router root"
                }
            }),
        );
    }
}

/// Build a Rust shadow `network.json` preview from Rust shadow collector output.
///
/// This keeps topology generation on the Rust side for preview/shadow use. It
/// mirrors the current Python network-mode rules closely enough for dry-run
/// comparison, and the active live run-cycle now enters Rust authority directly.
pub fn build_rust_network_json_shadow_payload(
    payload: &Value,
) -> (Value, Vec<Diagnostic>, Vec<Diagnostic>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let config = payload.get("config").unwrap_or(payload);
    let shadow_bundle = payload
        .get("shadow_bundle")
        .filter(|value| value.is_object())
        .unwrap_or(payload);
    let network_mode = get_network_mode(config);
    let preserve_network_config = boolish(config, "preserve_network_config");
    let (current_network, current_network_source, current_network_text) =
        load_current_network(payload, &mut warnings);

    let mut network_config = match network_mode.as_str() {
        "flat_no_parent" | "flat_router_root" => {
            if preserve_network_config {
                current_network.clone()
            } else {
                json!({})
            }
        }
        _ => current_network.clone(),
    };
    if !network_config.is_object() {
        network_config = json!({});
    }

    let rows_by_circuit = parse_shadow_rows(shadow_bundle);
    let meta = parse_shadow_meta(shadow_bundle);
    let mut node_math = Map::new();

    if let Some(routers) = config.get("routers").and_then(Value::as_array) {
        let root = network_config.as_object_mut().expect("network object");
        for router in routers
            .iter()
            .filter(|router| router.is_object())
            .filter(|router| {
                router
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true)
            })
        {
            let router_name = sval(router, "name");
            if router_name.is_empty() {
                continue;
            }
            if network_mode != "flat_no_parent" {
                ensure_router_node(root, router, is_deep_hierarchy_mode(&network_mode));
                if network_mode == "flat_router_root" {
                    if let Some(children) = router_children_mut(root, &router_name) {
                        children.clear();
                    }
                }
            }
            apply_pppoe_nodes(
                root,
                router,
                &network_mode,
                &rows_by_circuit,
                &meta,
                &mut node_math,
            );
            apply_dhcp_nodes(
                root,
                router,
                &network_mode,
                &rows_by_circuit,
                &meta,
                &mut node_math,
            );
            apply_hotspot_nodes(
                root,
                router,
                &network_mode,
                &rows_by_circuit,
                &meta,
                &mut node_math,
            );
        }
    }

    let (network_errors, network_warnings) = validate_network(&network_config);
    errors.extend(network_errors);
    warnings.extend(network_warnings);

    let network_text = match render_network_text(&network_config) {
        Ok(text) => text,
        Err(err) => {
            errors.push(Diagnostic::error(
                "rust_network_shadow_render_failed",
                Some("network".to_string()),
                format!("Rust network shadow builder failed to render network.json: {err}"),
            ));
            "{}\n".to_string()
        }
    };

    let status = if errors.is_empty() {
        "shadow_ready"
    } else {
        "blocked"
    };
    let result = json!({
        "mode": "rust_network_json_shadow",
        "status": status,
        "network_mode": network_mode,
        "preserve_network_config": preserve_network_config,
        "current_network_source": current_network_source,
        "current_network_text": current_network_text,
        "network": network_config,
        "network_text": network_text,
        "node_count": collect_node_names(&network_config).len(),
        "node_math": Value::Object(node_math),
    });

    (result, errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_router_child_nodes_from_shadow_bundle() {
        let payload = json!({
            "config": {
                "network_mode": "router_children",
                "routers": [{
                    "name": "RB5009",
                    "enabled": true,
                    "root_download_mbps": 115,
                    "root_upload_mbps": 115,
                    "pppoe": {
                        "enabled": true,
                        "per_plan_node": true,
                        "plan_node_name": "{profile}-{router}",
                        "factor_rules": [{"max_plan_mbps": 15, "download_factor": 0.31, "upload_factor": 0.31}]
                    },
                    "dhcp": {
                        "enabled": true,
                        "servers": [{"name": "LAN", "enabled": true, "download_factor": 0.5, "upload_factor": 0.5}]
                    },
                    "hotspot": {
                        "enabled": true,
                        "download_factor": 1.0,
                        "upload_factor": 1.0
                    }
                }]
            },
            "shadow_bundle": {
                "normalized_rows": [
                    {"Circuit Name": "juan", "Parent Node": "15M-RB5009", "Download Max Mbps": "15", "Upload Max Mbps": "15"},
                    {"Circuit Name": "DHCP-phone", "Parent Node": "DHCP-LAN-RB5009", "Download Max Mbps": "15", "Upload Max Mbps": "15"},
                    {"Circuit Name": "HS-AABBCC001122", "Parent Node": "HS-RB5009", "Download Max Mbps": "10", "Upload Max Mbps": "10"}
                ],
                "meta": [
                    {"circuit_name": "juan", "source_type": "PPP", "router": "RB5009", "profile": "15M"},
                    {"circuit_name": "DHCP-phone", "source_type": "DHCP", "router": "RB5009", "server": "LAN"},
                    {"circuit_name": "HS-AABBCC001122", "source_type": "HS", "router": "RB5009"}
                ]
            }
        });
        let (result, errors, warnings) = build_rust_network_json_shadow_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            result.get("status").and_then(Value::as_str),
            Some("shadow_ready")
        );
        let network = result.get("network").cloned().unwrap_or_else(|| json!({}));
        assert_eq!(
            network["RB5009"]["children"]["15M-RB5009"]["downloadBandwidthMbps"],
            json!(4.65)
        );
        assert_eq!(
            network["RB5009"]["children"]["DHCP-LAN-RB5009"]["downloadBandwidthMbps"],
            json!(7.5)
        );
        assert_eq!(
            network["RB5009"]["children"]["HS-RB5009"]["downloadBandwidthMbps"],
            json!(10.0)
        );
    }

    #[test]
    fn keeps_router_roots_only_in_flat_router_mode() {
        let payload = json!({
            "config": {
                "network_mode": "flat_router_root",
                "routers": [{
                    "name": "RB5009",
                    "enabled": true,
                    "root_download_mbps": 115,
                    "root_upload_mbps": 115,
                    "pppoe": {"enabled": true}
                }]
            },
            "shadow_bundle": {
                "normalized_rows": [
                    {"Circuit Name": "juan", "Parent Node": "RB5009", "Download Max Mbps": "15", "Upload Max Mbps": "15"}
                ],
                "meta": [
                    {"circuit_name": "juan", "source_type": "PPP", "router": "RB5009", "profile": "15M"}
                ]
            }
        });
        let (result, errors, warnings) = build_rust_network_json_shadow_payload(&payload);
        assert!(errors.is_empty(), "{errors:?}");
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(result["network"]["RB5009"]["children"], json!({}));
        assert_eq!(
            result["node_math"]["PPP-flat-RB5009"]["active_count"],
            json!(1)
        );
    }
}
