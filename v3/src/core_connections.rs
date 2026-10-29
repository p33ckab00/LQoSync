use crate::{
    core_analysis::Findings,
    core_diagnostics::Outcome,
    core_plan::{Binding, valid_name},
    core_routes::flag,
    routeros::{Inventory, Row},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
const MANGLE: &str = "/ip/firewall/mangle/print";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    SubscriberForward,
    RouterReply,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionPath {
    pub direction: Direction,
    pub wan: String,
    pub source_list: String,
    pub connection_mark: String,
    pub table: String,
    pub interface: String,
    pub classifier_row: String,
    pub routing_row: String,
    pub status: Outcome,
}
pub(crate) struct Analysis {
    pub paths: Vec<ConnectionPath>,
    pub findings: Vec<crate::core_diagnostics::Finding>,
    pub classifiers: HashSet<usize>,
    pub forward_routes: HashSet<usize>,
}
fn text<'a>(row: &'a Row, key: &str) -> &'a str {
    row.get(key).map(String::as_str).unwrap_or("")
}
fn reference(row: &Row, index: usize) -> String {
    row.get(".id")
        .cloned()
        .unwrap_or_else(|| format!("rule-{}", index + 1))
}
fn fields(row: &Row, specific: &[&str], reply_classifier: bool) -> bool {
    row.iter().all(|(key, value)| match key.as_str() {
        ".id" | "chain" | "action" | "disabled" | "dynamic" | "invalid" | "comment" | "bytes"
        | "packets" | "log" | "log-prefix" | "passthrough" => true,
        "src-address" | "dst-address" => value == "0.0.0.0/0",
        "dst-address-type" => {
            value.is_empty() || value == if reply_classifier { "local" } else { "!local" }
        }
        k => specific.contains(&k),
    })
}
pub(crate) fn analyze(
    inv: &Inventory,
    bindings: &[&Binding],
    interfaces: &HashMap<&str, &str>,
) -> Analysis {
    let rows = inv.tables.get(MANGLE).map(Vec::as_slice).unwrap_or(&[]);
    let mut producers = HashMap::<&str, Vec<usize>>::new();
    let mut consumers = HashMap::<&str, Vec<usize>>::new();
    for (n, row) in rows
        .iter()
        .enumerate()
        .filter(|(_, r)| text(r, "disabled") != "true")
    {
        if let Some(mark) = row.get("new-connection-mark") {
            producers.entry(mark).or_default().push(n);
        }
        if text(row, "action") == "mark-routing"
            && let Some(mark) = row.get("connection-mark")
            && mark != "no-mark"
        {
            consumers.entry(mark).or_default().push(n);
        }
    }
    let by_list: HashMap<_, _> = bindings.iter().map(|b| (b.list.as_str(), *b)).collect();
    let by_table: HashMap<_, _> = bindings.iter().map(|b| (b.table.as_str(), *b)).collect();
    let mut interface_rows = HashMap::<&str, Option<&Row>>::new();
    for row in inv.tables.get("/interface/print").into_iter().flatten() {
        if let Some(name) = row.get("name") {
            interface_rows
                .entry(name)
                .and_modify(|r| *r = None)
                .or_insert(Some(row));
        }
    }
    let mut result = Analysis {
        paths: vec![],
        findings: vec![],
        classifiers: HashSet::new(),
        forward_routes: HashSet::new(),
    };
    let mut findings = Findings(vec![]);
    let mut reply_routes = HashSet::new();
    // Preserve RouterOS rule order; connection classifiers must precede forward consumers.
    for (n, producer) in rows
        .iter()
        .enumerate()
        .filter(|(_, r)| text(r, "disabled") != "true" && r.contains_key("new-connection-mark"))
    {
        if result.paths.len() >= 512 {
            findings.add(
                Outcome::Unsupported,
                "connection-path-limit",
                "",
                MANGLE,
                producer,
                "Additional connection paths were omitted (limit 512); review is incomplete",
            );
            break;
        }
        let mark = text(producer, "new-connection-mark");
        let candidates = consumers.get(mark).map(Vec::as_slice).unwrap_or(&[]);
        let consumer_index = candidates.first().copied();
        let empty = Row::new();
        let consumer = consumer_index.map(|n| &rows[n]).unwrap_or(&empty);
        let reply = text(producer, "src-address-list").is_empty();
        let binding = if reply {
            by_table.get(text(consumer, "new-routing-mark"))
        } else {
            by_list.get(text(producer, "src-address-list"))
        }
        .copied();
        let wan = binding.map(|b| b.wan.as_str()).unwrap_or("");
        let expected_interface = interfaces.get(wan).copied().unwrap_or("");
        let mut issues = Findings(vec![]);
        let mut issue = |status, code, row: &Row, message: &str| {
            issues.add(status, code, wan, MANGLE, row, message)
        };
        if !valid_name(mark) || mark == "no-mark" {
            issue(
                Outcome::Unsupported,
                "connection-mark-name",
                producer,
                "Connection mark must be an exact positive named mark",
            );
        }
        if producers.get(mark).is_none_or(|p| p.len() != 1) {
            issue(
                Outcome::Conflict,
                "connection-producer-duplicate",
                producer,
                "Connection mark has more than one possible producer",
            );
        }
        if candidates.len() != 1 {
            issue(
                Outcome::Conflict,
                "connection-consumer-count",
                producer,
                "Connection mark requires exactly one enabled routing consumer",
            );
        }
        if binding.is_none() {
            issue(
                Outcome::Unsupported,
                "connection-mapping-required",
                consumer,
                "Connection path requires an explicit list/table/WAN relationship",
            );
        }
        for row in [producer, consumer] {
            if !matches!(flag(row, "disabled", false), Ok(false)) {
                issue(
                    Outcome::Unavailable,
                    "connection-status",
                    row,
                    "Connection rule has unknown or disabled status",
                );
            }
            if !matches!(flag(row, "invalid", false), Ok(false))
                || !matches!(flag(row, "dynamic", false), Ok(false))
            {
                issue(
                    Outcome::Conflict,
                    "connection-rule-invalid",
                    row,
                    "Connection rule is invalid, dynamic or has unknown status",
                );
            }
        }
        if text(producer, "chain") != "prerouting"
            || text(producer, "action") != "mark-connection"
            || text(producer, "connection-state") != "new"
            || text(producer, "connection-mark") != "no-mark"
            || !matches!(flag(producer, "passthrough", true), Ok(true))
        {
            issue(
                Outcome::Unsupported,
                "connection-classifier",
                producer,
                "Classifier must mark new unmarked prerouting connections and continue processing",
            );
        }
        let producer_fields = if reply {
            vec![
                "connection-state",
                "connection-mark",
                "new-connection-mark",
                "in-interface",
            ]
        } else {
            vec![
                "connection-state",
                "connection-mark",
                "new-connection-mark",
                "src-address-list",
            ]
        };
        let consumer_fields = if reply {
            vec!["connection-mark", "new-routing-mark"]
        } else {
            vec!["connection-mark", "new-routing-mark", "src-address-list"]
        };
        if !fields(producer, &producer_fields, reply) || !fields(consumer, &consumer_fields, false)
        {
            issue(
                Outcome::Unsupported,
                "connection-matcher",
                producer,
                "Unsupported connection-path matcher requires review",
            );
        }
        if text(consumer, "action") != "mark-routing"
            || text(consumer, "chain") != if reply { "output" } else { "prerouting" }
            || !matches!(flag(consumer, "passthrough", true), Ok(false))
        {
            issue(
                Outcome::Unsupported,
                "connection-routing-consumer",
                consumer,
                "Routing consumer must use the correct chain and passthrough=false",
            );
        }
        if let Some(binding) = binding {
            if text(consumer, "new-routing-mark") != binding.table {
                issue(
                    Outcome::Conflict,
                    "connection-table",
                    consumer,
                    "Connection path targets a different WAN routing table",
                );
            }
            if reply {
                let interface = interface_rows.get(expected_interface).and_then(|r| *r);
                if expected_interface.is_empty()
                    || text(producer, "in-interface") != expected_interface
                    || !interface.is_some_and(|r| {
                        matches!(flag(r, "running", false), Ok(true))
                            && matches!(flag(r, "disabled", false), Ok(false))
                    })
                {
                    issue(
                        Outcome::Conflict,
                        "connection-return-interface",
                        producer,
                        "WAN ingress mark must return through the same active WAN interface",
                    );
                }
            } else {
                if text(consumer, "src-address-list") != binding.list {
                    issue(
                        Outcome::Conflict,
                        "connection-source-list",
                        consumer,
                        "Forward routing consumer must match the same positive subscriber source list",
                    );
                }
                if consumer_index.is_none_or(|route| route <= n) {
                    issue(
                        Outcome::Conflict,
                        "connection-rule-order",
                        consumer,
                        "Forward classifier must precede its routing consumer",
                    );
                }
            }
        }
        let status = issues
            .0
            .iter()
            .map(|f| f.status)
            .max()
            .unwrap_or(Outcome::Pass);
        if status == Outcome::Pass {
            result.classifiers.insert(n);
            if !reply {
                result
                    .forward_routes
                    .insert(consumer_index.expect("validated consumer"));
            } else {
                reply_routes.insert(consumer_index.expect("validated reply consumer"));
            }
        }
        for f in issues.0 {
            findings.add(
                f.status,
                &f.code,
                &f.wan,
                &f.table,
                &[(".id".into(), f.row)].into_iter().collect(),
                f.message,
            );
        }
        result.paths.push(ConnectionPath {
            direction: if reply {
                Direction::RouterReply
            } else {
                Direction::SubscriberForward
            },
            wan: wan.into(),
            source_list: if reply {
                String::new()
            } else {
                text(producer, "src-address-list").into()
            },
            connection_mark: mark.into(),
            table: text(consumer, "new-routing-mark").into(),
            interface: expected_interface.into(),
            classifier_row: reference(producer, n),
            routing_row: consumer_index
                .map(|n| reference(consumer, n))
                .unwrap_or_default(),
            status,
        });
    }
    for (n, row) in rows
        .iter()
        .enumerate()
        .filter(|(_, r)| text(r, "disabled") != "true")
    {
        if text(row, "action") == "mark-connection" && !row.contains_key("new-connection-mark") {
            findings.add(Outcome::Unsupported,"connection-mark-clear","",MANGLE,row,"Connection rule has no explicit new mark; implicit clearing or unknown behavior requires review");
        }
        if text(row, "chain") == "output"
            && !reply_routes.contains(&n)
            && !["log", "passthrough"].contains(&text(row, "action"))
        {
            findings.add(
                Outcome::Unsupported,
                "connection-output-policy",
                "",
                MANGLE,
                row,
                "Output action is outside the validated router-reply mark path",
            );
        }
    }
    for (mark, indices) in consumers {
        if !producers.contains_key(mark) {
            findings.add(
                Outcome::Unsupported,
                "connection-producer-missing",
                "",
                MANGLE,
                &rows[indices[0]],
                "Connection-mark matcher has no enabled producer",
            );
        }
    }
    result.findings = findings.0;
    result
}
