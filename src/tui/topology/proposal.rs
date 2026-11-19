//! The Builder edit proposal popup: what the confirm commits.

use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::Value;

/// The most moves that the popup lists.
const LISTED: usize = 8;

/// The node name for `id` from the proposal tree, then the current tree, else the ID.
fn namer<'a>(p: &'a Value, current: &'a [Value]) -> impl Fn(&Value) -> String + 'a {
    move |id: &Value| {
        let Some(id) = id.as_str() else {
            return "root".into();
        };
        p["nodes"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(current)
            .find(|n| n["id"].as_str() == Some(id))
            .and_then(|n| n["name"].as_str())
            .unwrap_or(id)
            .to_string()
    }
}

/// One line that says what the edit changes in the tree.
fn change(e: &Value, name: &dyn Fn(&Value) -> String, sources: &[Value]) -> String {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let source = |id: &Value| {
        sources
            .iter()
            .find(|s| s["id"] == *id)
            .and_then(|s| s["name"].as_str())
            .map_or_else(|| text(id), str::to_string)
    };
    let who = || {
        e["identity"]["account_ref"]
            .as_str()
            .map_or_else(|| text(&e["subscriber_id"]), str::to_string)
    };
    match e["kind"].as_str().unwrap_or_default() {
        "speed.defaults" => "speed defaults".into(),
        "group.factor" => format!("factor of {}", source(&e["source_id"])),
        "rebalance" if e["source_id"].is_null() => "rebalance all groups".into(),
        "rebalance" => format!("rebalance {}", source(&e["source_id"])),
        "subscriber.pin" if e["wan_node_id"].is_null() => format!("unpin {}", who()),
        "subscriber.pin" => format!("pin {} to {}", who(), name(&e["wan_node_id"])),
        "node.remove" => format!("remove {}", name(&e["node_id"])),
        "node.move" => format!("move {} below {}", name(&e["node_id"]), name(&e["parent"])),
        "node.update" => format!("edit {}", name(&e["node_id"])),
        "instance.add" => format!(
            "new instance of {} below {}",
            text(&e["source_id"]),
            name(&e["parent"])
        ),
        _ => format!("create {}", text(&e["name"])),
    }
}

pub(super) fn lines(p: &Value, current: &[Value], sources: &[Value]) -> Vec<Line<'static>> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let name = namer(p, current);
    let moves = p["moves"].as_array().map_or(&[][..], Vec::as_slice);
    let count = |k: &str| {
        p[k].as_array()
            .map_or_else(|| p[k].as_u64().unwrap_or(0) as usize, Vec::len)
    };
    let mut out = vec![
        format!("edit: {}", text(&p["edit"]["kind"])),
        change(&p["edit"], &name, sources),
        format!("{} moves", moves.len()),
    ];
    for m in moves.iter().take(LISTED) {
        let who = m["label"]
            .as_str()
            .map_or_else(|| text(&m["subscriber_id"]), str::to_string);
        out.push(format!(
            "  {who}  {} -> {}",
            name(&m["from"]),
            name(&m["to"])
        ));
    }
    if moves.len() > LISTED {
        out.push(format!("  and {} more", moves.len() - LISTED));
    }
    out.push(format!(
        "{} cleared  {} assigned  {} new records",
        count("cleared"),
        count("assigned"),
        count("new_records")
    ));
    let pair = |v: &Value| {
        let n = |x: &Value| {
            let f = x.as_f64().unwrap_or(0.0);
            if f.fract() == 0.0 {
                format!("{f:.0}")
            } else {
                format!("{f}")
            }
        };
        format!("{}/{}", n(&v[0]), n(&v[1]))
    };
    if let Some(changed) = p["rates_changed"].as_u64().filter(|&n| n > 0) {
        out.push(format!("{changed} subscribers change"));
        for r in p["rates"].as_array().into_iter().flatten() {
            let who = r["label"]
                .as_str()
                .map_or_else(|| text(&r["subscriber_id"]), str::to_string);
            out.push(format!(
                "  {who}  min {} -> {}  max {} -> {}",
                pair(&r["before"]["min_mbps"]),
                pair(&r["after"]["min_mbps"]),
                pair(&r["before"]["max_mbps"]),
                pair(&r["after"]["max_mbps"])
            ));
        }
    }
    for c in p["capacities"].as_array().into_iter().flatten() {
        out.push(format!(
            "  {}  {} -> {} Mbps",
            name(&c["node_id"]),
            pair(&c["before_mbps"]),
            pair(&c["after_mbps"])
        ));
    }
    for w in p["warnings"].as_array().into_iter().flatten() {
        out.push(format!("! {}", text(w)));
    }
    out.push(String::new());
    out.push("[Enter] confirm  [Esc] cancel".into());
    out.into_iter().map(Line::from).collect()
}

pub(super) fn draw(
    p: &Value,
    (current, sources): (&[Value], &[Value]),
    f: &mut Frame,
    area: Rect,
    palette: &Palette,
) {
    let lines = lines(p, current, sources);
    super::plan_form::popup(f, area, lines, "Edit proposal", palette);
}

#[cfg(test)]
mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::{Value, json};

    fn ops(v: &mut TopologyView) -> Vec<(String, Value)> {
        v.take_outbox()
    }

    pub(crate) fn proposed() -> TopologyView {
        let mut v = super::super::builder::tests::sample();
        v.on_answer(ApiAnswer {
            op: "topology.edit.propose".into(),
            result: Ok(json!({
                "proposal_id": "edit-2-1", "expires_at": 0,
                "edit": {"kind": "instance.add", "source_id": "src-g", "parent": "wan2"},
                "nodes": [], "sources": ["src-g"],
                "moves": [
                    {"subscriber_id": "a", "label": "alice", "from": "g", "to": "g2"},
                    {"subscriber_id": "b", "label": null, "from": "g", "to": "g2"}
                ],
                "cleared": ["c"], "assigned": 4, "new_records": 1
            })),
        });
        v
    }

    #[test]
    fn a_proposal_answer_opens_the_popup_with_moves_and_counts() {
        let v = proposed();
        assert!(v.capturing());
        let text = render(&v, 100, 30);
        for want in [
            "instance.add",
            "2 moves",
            "alice  g -> g2",
            "b  g -> g2",
            "1 cleared",
            "4 assigned",
            "1 new records",
            "[Enter] confirm  [Esc] cancel",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn enter_confirms_and_esc_cancels_the_proposal() {
        let mut v = proposed();
        ops(&mut v);
        assert!(v.on_key(KeyCode::Enter));
        assert!(!v.capturing());
        assert_eq!(
            ops(&mut v),
            vec![(
                "topology.edit.confirm".to_string(),
                json!({"proposal_id": "edit-2-1"})
            )]
        );
        v.on_answer(ApiAnswer {
            op: "topology.edit.confirm".into(),
            result: Ok(json!({"revision": 3})),
        });
        let sent: Vec<String> = ops(&mut v).into_iter().map(|(op, _)| op).collect();
        for want in [
            "topology.preview.refresh",
            "status",
            "topology.policies",
            "topology.tree",
        ] {
            assert!(sent.contains(&want.to_string()), "{want}: {sent:?}");
        }
        let mut v = proposed();
        ops(&mut v);
        v.on_key(KeyCode::Esc);
        assert!(!v.capturing());
        assert_eq!(
            ops(&mut v),
            vec![(
                "topology.edit.cancel".to_string(),
                json!({"proposal_id": "edit-2-1"})
            )]
        );
    }

    #[test]
    fn a_stale_confirm_shows_the_conflict_message_and_reloads() {
        let mut v = proposed();
        v.on_key(KeyCode::Enter);
        ops(&mut v);
        v.on_answer(ApiAnswer {
            op: "topology.edit.confirm".into(),
            result: Err("Conflict: the proposal is stale, propose again".into()),
        });
        assert!(render(&v, 120, 30).contains("changed by another operator, try again"));
        let sent: Vec<String> = ops(&mut v).into_iter().map(|(op, _)| op).collect();
        assert!(sent.contains(&"topology.tree".to_string()), "{sent:?}");
    }

    #[test]
    fn the_popup_names_nodes_and_the_tree_change() {
        let mut v = super::super::builder::tests::sample();
        v.on_answer(ApiAnswer {
            op: "topology.edit.propose".into(),
            result: Ok(json!({
                "proposal_id": "p", "edit": {"kind": "node.remove", "node_id": "lq3-aaaaaaaaaaaaaaaa"},
                "nodes": [{"id": "lq3-bbbbbbbbbbbbbbbb", "name": "Gold WAN1"}],
                "moves": [{"subscriber_id": "a", "label": "alice",
                           "from": "lq3-aaaaaaaaaaaaaaaa", "to": "lq3-bbbbbbbbbbbbbbbb"}],
                "cleared": [], "assigned": 0, "new_records": 0, "sources": []
            })),
        });
        // The removed node is not in the candidate tree; the current tree names it.
        v.on_answer(ApiAnswer {
            op: "topology.tree".into(),
            result: Ok(json!({"revision": 2, "sources": [], "nodes": [
                {"id": "lq3-aaaaaaaaaaaaaaaa", "name": "Gold WAN2", "parent": null, "kind": "group"},
                {"id": "lq3-bbbbbbbbbbbbbbbb", "name": "Gold WAN1", "parent": null, "kind": "group"}
            ]})),
        });
        let text = render(&v, 80, 24);
        assert!(text.contains("remove Gold WAN2"), "{text}");
        assert!(text.contains("alice  Gold WAN2 -> Gold WAN1"), "{text}");
    }

    #[test]
    fn popup_lines_name_rebalance_and_pin_changes() {
        let nodes = vec![json!({"id": "wan2", "name": "WAN2"})];
        let sources = vec![json!({"id": "src-g", "name": "Gold"})];
        let text = |edit: Value| {
            super::lines(
                &json!({"edit": edit, "moves": [], "nodes": []}),
                &nodes,
                &sources,
            )
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n")
        };
        assert!(
            text(json!({"kind": "rebalance", "source_id": "src-g"})).contains("rebalance Gold")
        );
        assert!(
            text(json!({"kind": "rebalance", "source_id": null})).contains("rebalance all groups")
        );
        assert!(
            text(json!({"kind": "subscriber.pin", "identity": {"account_ref": "acct-1"}, "wan_node_id": "wan2"}))
                .contains("pin acct-1 to WAN2")
        );
        assert!(
            text(
                json!({"kind": "subscriber.pin", "subscriber_id": "tm-sub-9", "wan_node_id": null})
            )
            .contains("unpin tm-sub-9")
        );
    }

    #[test]
    fn popup_lines_show_rates_capacities_and_warnings() {
        let nodes = vec![json!({"id": "g", "name": "Gold WAN1"})];
        let sources = vec![json!({"id": "src-g", "name": "Gold"})];
        let p = json!({"edit": {"kind": "group.factor", "source_id": "src-g", "down": "0.3"},
            "moves": [], "nodes": [], "rates_changed": 1,
            "rates": [{"subscriber_id": "a", "label": "alice",
                       "before": {"min_mbps": [5.0, 1.0], "max_mbps": [50.0, 10.0]},
                       "after": {"min_mbps": [10.0, 2.0], "max_mbps": [50.0, 10.0]}}],
            "capacities": [{"node_id": "g", "before_mbps": [107.5, 5.0], "after_mbps": [150.0, 5.0]}],
            "warnings": ["the legacy rounded rules of 2 profiles are replaced"]});
        let text = super::lines(&p, &nodes, &sources)
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        for want in [
            "factor of Gold",
            "1 subscribers change",
            "alice  min 5/1 -> 10/2  max 50/10 -> 50/10",
            "Gold WAN1  107.5/5 -> 150/5 Mbps",
            "the legacy rounded rules of 2 profiles are replaced",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
        let speed = json!({"edit": {"kind": "speed.defaults"}, "moves": [], "nodes": []});
        assert!(
            super::lines(&speed, &nodes, &sources)
                .iter()
                .any(|l| l.to_string() == "speed defaults")
        );
    }

    #[test]
    fn the_proposal_popup_fits_80_columns() {
        let text = render(&proposed(), 80, 24);
        assert!(text.contains("[Enter] confirm  [Esc] cancel"), "{text}");
        assert!(text.contains("alice  g -> g2"), "{text}");
    }
}
