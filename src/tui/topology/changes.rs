//! The read-only Changes view: revision evidence, rounded nodes, and conflicts.

use super::TopologyView;
use super::builder::mbps;
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde_json::Value;

fn pair(v: &Value) -> String {
    format!("{}/{}", mbps(&v[0]), mbps(&v[1]))
}

pub(super) fn draw(v: &TopologyView, f: &mut Frame, area: Rect, p: &Palette) {
    let mut lines = Vec::new();
    if let Some(r) = v.revision() {
        lines.push(Line::from(format!("registry revision {r}")));
    }
    match v.preview() {
        None => lines.push(Line::from("no preview")),
        Some(preview) => {
            lines.push(Line::from(match preview["generation"].as_u64() {
                Some(g) => format!("preview generation {g}"),
                None => "preview generation none".into(),
            }));
            lines.push(Line::from(""));
            lines.push(Line::from("Rounded node exports").style(p.role(Role::Secondary)));
            let rounded = preview["rounded"].as_array().map_or(&[][..], Vec::as_slice);
            if rounded.is_empty() {
                lines.push(Line::from("  none"));
            }
            for r in rounded {
                lines.push(Line::from(format!(
                    "  {}: {} -> {}",
                    r["node_id"].as_str().unwrap_or_default(),
                    pair(&r["calculated_mbps"]),
                    pair(&r["exported_mbps"])
                )));
            }
            lines.push(Line::from(""));
            let conflicts = preview["conflicts"]
                .as_array()
                .map_or(&[][..], Vec::as_slice);
            if conflicts.is_empty() {
                lines.push(Line::from("no conflicts").style(p.role(Role::Ok)));
            } else {
                lines.push(
                    Line::from(format!("{} critical conflict(s)", conflicts.len()))
                        .style(p.role(Role::Error)),
                );
                for c in conflicts {
                    lines.push(Line::from(format!(
                        "  {}  {}  {}",
                        c["kind"].as_str().unwrap_or_default(),
                        c["subject"].as_str().unwrap_or_default(),
                        c["detail"].as_str().unwrap_or_default()
                    )));
                }
            }
        }
    }
    f.render_widget(
        Paragraph::new(lines)
            .style(p.role(Role::Base))
            .block(crate::tui::design::panel("Changes", p)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::super::tests::render;
    use super::super::*;
    use crate::tui::api::ApiAnswer;
    use ratatui::crossterm::event::KeyCode;
    use serde_json::{Value, json};

    fn view(preview: Value) -> TopologyView {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Ok(json!({"registry_revision": 4, "enrolled": true})),
        });
        v.on_answer(ApiAnswer {
            op: "topology.preview.refresh".into(),
            result: Ok(preview),
        });
        v.on_key(KeyCode::Char('c'));
        v
    }

    #[test]
    fn changes_list_rounded_nodes_and_conflicts() {
        let v = view(
            json!({"collected_at": 0, "state": "conflicts", "generation": null, "registry_revision": 4,
            "nodes": [], "subscribers": [], "inventory": [],
            "rounded": [{"node_id": "g", "calculated_mbps": [52.5, 52.5], "exported_mbps": [52.0, 52.0]}],
            "conflicts": [{"kind": "identity", "subject": "dhcp-LAN/*l0", "detail": "DuplicateReference"}]}),
        );
        let text = render(&v, 160, 24);
        for want in [
            "registry revision 4",
            "identity",
            "dhcp-LAN/*l0",
            "DuplicateReference",
            "g: 52.5/52.5 -> 52/52",
        ] {
            assert!(text.contains(want), "{want}: {text}");
        }
    }

    #[test]
    fn changes_show_no_conflicts_when_the_preview_is_ok() {
        let v = view(
            json!({"collected_at": 0, "state": "ok", "generation": 7, "registry_revision": 4,
            "nodes": [], "subscribers": [], "inventory": [], "rounded": [], "conflicts": []}),
        );
        let text = render(&v, 160, 24);
        assert!(text.contains("no conflicts"), "{text}");
        assert!(text.contains("generation 7"), "{text}");
    }
}
