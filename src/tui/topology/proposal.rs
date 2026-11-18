//! The Builder edit proposal popup: what the confirm commits.

use crate::tui::theme::Palette;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use serde_json::Value;

/// The most moves that the popup lists.
const LISTED: usize = 8;

pub(super) fn lines(p: &Value) -> Vec<Line<'static>> {
    let text = |v: &Value| v.as_str().unwrap_or_default().to_string();
    let moves = p["moves"].as_array().map_or(&[][..], Vec::as_slice);
    let count = |k: &str| {
        p[k].as_array()
            .map_or_else(|| p[k].as_u64().unwrap_or(0) as usize, Vec::len)
    };
    let mut out = vec![
        format!("edit: {}", text(&p["edit"]["kind"])),
        format!("{} moves", moves.len()),
    ];
    for m in moves.iter().take(LISTED) {
        let who = m["label"]
            .as_str()
            .map_or_else(|| text(&m["subscriber_id"]), str::to_string);
        out.push(format!(
            "  {who}  {} -> {}",
            text(&m["from"]),
            text(&m["to"])
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
    out.push(String::new());
    out.push("[Enter] confirm  [Esc] cancel".into());
    out.into_iter().map(Line::from).collect()
}

pub(super) fn draw(p: &Value, f: &mut Frame, area: Rect, palette: &Palette) {
    super::plan_form::popup(f, area, lines(p), "Edit proposal", palette);
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
    fn the_proposal_popup_fits_80_columns() {
        let text = render(&proposed(), 80, 24);
        assert!(text.contains("[Enter] confirm  [Esc] cancel"), "{text}");
        assert!(text.contains("alice  g -> g2"), "{text}");
    }
}
