//! The Topology Manager route. Read-only views of the service preview, through the service API.

use crate::tui::api::{ApiAnswer, ApiLink};
use crate::tui::theme::{Palette, Role};
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use serde_json::{Value, json};

/// A cached preview older than this is refreshed once when the route opens.
pub const STALE_AFTER_SECS: u64 = 300;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Builder,
    Assignments,
    Policies,
    Changes,
}

/// The focused Builder panel in a narrow terminal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Panel {
    Inventory,
    #[default]
    Tree,
    Details,
}

#[derive(Default)]
pub struct TopologyView {
    /// Depth-first tree order of the preview nodes. Rebuilt only when the preview changes.
    tree: Vec<(usize, usize)>,
    node_selected: usize,
    sub_selected: usize,
    focus: Panel,
    link: Option<ApiLink>,
    outbox: Vec<(String, Value)>,
    pending: usize,
    opened: bool,
    refreshing: bool,
    now: u64,
    status: Option<Value>,
    preview: Option<Value>,
    policies: Option<Value>,
    error: Option<String>,
    picker: Option<picker::Picker>,
    form: Option<plan_form::PlanForm>,
    /// The plan that waits for a y/n answer before delete.
    confirm_delete: Option<(String, String)>,
    plan_selected: usize,
    pub screen: Screen,
}

/// The service answers a stale revision with an error text that starts with this code.
fn is_conflict(error: &str) -> bool {
    error.starts_with("Conflict:")
}

impl TopologyView {
    pub fn with_link(link: ApiLink) -> Self {
        Self {
            link: Some(link),
            ..Self::default()
        }
    }

    fn send(&mut self, op: &str) {
        self.outbox.push((op.to_string(), json!({})));
        self.pending += 1;
    }

    fn refresh(&mut self) {
        if !self.refreshing {
            self.refreshing = true;
            self.send("topology.preview.refresh");
            // An enrollment or a service restart can change the state at any time.
            self.send("status");
        }
    }

    /// After an edit or a conflict: ask again for status, preview, and policies.
    fn reload(&mut self) {
        self.refreshing = false;
        self.refresh();
        self.send("topology.policies");
    }

    /// True while a dialog of this route has all keys.
    pub fn capturing(&self) -> bool {
        self.picker.is_some() || self.form.is_some() || self.confirm_delete.is_some()
    }

    fn plans(&self) -> &[Value] {
        self.policies
            .as_ref()
            .and_then(|p| p["plans"].as_array())
            .map_or(&[], Vec::as_slice)
    }

    fn selected_plan(&self) -> Option<&Value> {
        self.plans().get(self.plan_selected)
    }

    /// Queue an edit with the revision from the last status.
    fn send_edit(&mut self, op: &str, mut args: Value) {
        let Some(revision) = self.revision() else {
            self.error = Some("the registry revision is unknown, press r".into());
            return;
        };
        args["expected_revision"] = json!(revision);
        self.outbox.push((op.to_string(), args));
        self.pending += 1;
    }

    fn dialog_key(&mut self, code: KeyCode) {
        if let Some((id, _)) = &self.confirm_delete {
            match code {
                KeyCode::Char('y') => {
                    let args = json!({"plan_id": id});
                    self.confirm_delete = None;
                    self.send_edit("plan.delete", args);
                }
                KeyCode::Char('n') | KeyCode::Esc => self.confirm_delete = None,
                _ => {}
            }
            return;
        }
        if let Some(form) = self.form.as_mut() {
            let plans = self
                .policies
                .as_ref()
                .and_then(|p| p["plans"].as_array())
                .map_or(&[][..], Vec::as_slice);
            match form.key(code, plans) {
                plan_form::Step::Stay => {}
                plan_form::Step::Cancel => self.form = None,
                plan_form::Step::Send(op, args) => {
                    self.form = None;
                    self.send_edit(op, args);
                }
            }
            return;
        }
        self.picker_key(code);
    }

    fn picker_key(&mut self, code: KeyCode) {
        let Some(picker) = self.picker.as_mut() else {
            return;
        };
        let count = picker::entries(self.policies.as_ref()).len();
        match code {
            KeyCode::Up => picker.selected = picker.selected.saturating_sub(1),
            KeyCode::Down => picker.selected = (picker.selected + 1).min(count - 1),
            KeyCode::Esc => self.picker = None,
            KeyCode::Enter => {
                let selected = picker.selected;
                self.picker = None;
                let plan_id = picker::entries(self.policies.as_ref())
                    .swap_remove(selected)
                    .0;
                let identity = self
                    .selected_subscriber()
                    .map(|s| s["identities"][0].clone())
                    .filter(|i| !i.is_null());
                match identity {
                    Some(identity) => self.send_edit(
                        "subscriber.plan",
                        json!({"identity": identity, "plan_id": plan_id}),
                    ),
                    None => self.error = Some("the subscriber has no identity to edit".into()),
                }
            }
            _ => {}
        }
    }

    /// Requests that wait for the API link. Tests read them directly.
    pub fn take_outbox(&mut self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.outbox)
    }

    pub fn preview(&self) -> Option<&Value> {
        self.preview.as_ref()
    }

    fn policies(&self) -> Option<&Value> {
        self.policies.as_ref()
    }

    fn revision(&self) -> Option<u64> {
        self.status.as_ref()?["registry_revision"].as_u64()
    }

    fn nodes(&self) -> &[Value] {
        self.preview
            .as_ref()
            .and_then(|p| p["nodes"].as_array())
            .map_or(&[], Vec::as_slice)
    }

    fn subscribers(&self) -> &[Value] {
        self.preview
            .as_ref()
            .and_then(|p| p["subscribers"].as_array())
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn selected_subscriber(&self) -> Option<&Value> {
        self.subscribers().get(self.sub_selected)
    }

    fn node_label(&self, id: &str) -> String {
        self.nodes()
            .iter()
            .find(|n| n["id"].as_str() == Some(id))
            .and_then(|n| n["label"].as_str())
            .unwrap_or(id)
            .to_string()
    }

    fn selected_node(&self) -> Option<&Value> {
        let &(_, i) = self.tree.get(self.node_selected)?;
        self.nodes().get(i)
    }

    fn set_preview(&mut self, v: Value) {
        let selected = self
            .selected_node()
            .and_then(|n| n["id"].as_str())
            .map(str::to_string);
        let subscriber = self
            .selected_subscriber()
            .and_then(|s| s["id"].as_str())
            .map(str::to_string);
        let old_row = self.sub_selected;
        self.preview = Some(v);
        self.tree = builder::index(self.nodes());
        self.node_selected = selected
            .and_then(|id| {
                self.tree
                    .iter()
                    .position(|&(_, i)| self.nodes()[i]["id"].as_str() == Some(id.as_str()))
            })
            .unwrap_or(0)
            .min(self.tree.len().saturating_sub(1));
        // Keep the same subscriber, or the nearest row when it is gone.
        self.sub_selected = subscriber
            .and_then(|id| {
                self.subscribers()
                    .iter()
                    .position(|s| s["id"].as_str() == Some(id.as_str()))
            })
            .unwrap_or(old_row)
            .min(self.subscribers().len().saturating_sub(1));
    }

    /// Open the route once, send queued requests, and apply ready answers. True when the view changed.
    pub fn tick(&mut self, active: bool, now: u64) -> bool {
        self.now = now;
        let mut changed = false;
        if !active {
            // Opening the route again asks for a fresh status and preview age.
            self.opened = false;
        }
        if active && !self.opened {
            self.opened = true;
            self.send("status");
            self.send("topology.preview.get");
            changed = true;
        }
        let Some(link) = self.link.take() else {
            return changed;
        };
        let mut answers = Vec::new();
        while let Some(a) = link.try_recv() {
            answers.push(a);
        }
        for a in answers {
            self.on_answer(a);
            changed = true;
        }
        for (op, args) in self.take_outbox() {
            link.send(&op, args);
        }
        self.link = Some(link);
        changed
    }

    pub fn on_answer(&mut self, answer: ApiAnswer) {
        self.pending = self.pending.saturating_sub(1);
        match (answer.op.as_str(), answer.result) {
            ("status", Ok(v)) => {
                self.status = Some(v);
                self.error = None;
            }
            ("topology.preview.get", Ok(v)) => {
                let age = v["collected_at"]
                    .as_u64()
                    .map(|t| self.now.saturating_sub(t));
                if !v.is_null() {
                    self.set_preview(v);
                }
                if age.is_none_or(|a| a > STALE_AFTER_SECS) {
                    self.refresh();
                }
            }
            ("topology.preview.refresh", result) => {
                self.refreshing = false;
                match result {
                    Ok(v) => {
                        self.set_preview(v);
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            ("topology.policies", Ok(v)) => {
                self.policies = Some(v);
                self.plan_selected = self.plan_selected.min(self.plans().len().saturating_sub(1));
            }
            ("subscriber.plan" | "plan.create" | "plan.update" | "plan.delete", Ok(_)) => {
                self.error = None;
                self.reload();
            }
            ("subscriber.plan" | "plan.create" | "plan.update" | "plan.delete", Err(e))
                if is_conflict(&e) =>
            {
                self.error = Some("changed by another operator, try again".into());
                self.reload();
            }
            (_, Err(e)) => self.error = Some(e),
            _ => {}
        }
    }

    /// Route keys. True when the key belongs to this route.
    pub fn on_key(&mut self, code: KeyCode) -> bool {
        if self.capturing() {
            self.dialog_key(code);
            return true;
        }
        match code {
            KeyCode::Enter
                if self.screen == Screen::Assignments && self.selected_subscriber().is_some() =>
            {
                self.picker = Some(picker::Picker::default());
                if self.policies.is_none() {
                    self.send("topology.policies");
                }
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('b') => self.screen = Screen::Builder,
            KeyCode::Char('a') => self.screen = Screen::Assignments,
            KeyCode::Char('p') => {
                self.screen = Screen::Policies;
                if self.policies.is_none() {
                    self.send("topology.policies");
                }
            }
            KeyCode::Char('c') => self.screen = Screen::Changes,
            KeyCode::Char('n') if self.screen == Screen::Policies && self.policies.is_some() => {
                self.form = Some(plan_form::PlanForm::default());
            }
            KeyCode::Char('e') if self.screen == Screen::Policies => {
                if let Some(plan) = self.selected_plan() {
                    self.form = Some(plan_form::PlanForm::edit(plan));
                }
            }
            KeyCode::Char('d') if self.screen == Screen::Policies => {
                if let Some(plan) = self.selected_plan() {
                    let id = plan["id"].as_str().unwrap_or_default().to_string();
                    let name = plan["name"].as_str().unwrap_or_default().to_string();
                    self.confirm_delete = Some((id, name));
                }
            }
            KeyCode::Up if self.screen == Screen::Policies => {
                self.plan_selected = self.plan_selected.saturating_sub(1);
            }
            KeyCode::Down if self.screen == Screen::Policies => {
                self.plan_selected =
                    (self.plan_selected + 1).min(self.plans().len().saturating_sub(1));
            }
            KeyCode::Up if self.screen == Screen::Assignments => {
                self.sub_selected = self.sub_selected.saturating_sub(1);
            }
            KeyCode::Down if self.screen == Screen::Assignments => {
                self.sub_selected =
                    (self.sub_selected + 1).min(self.subscribers().len().saturating_sub(1));
            }
            KeyCode::Up if self.screen == Screen::Builder => {
                self.node_selected = self.node_selected.saturating_sub(1);
            }
            KeyCode::Down if self.screen == Screen::Builder => {
                self.node_selected =
                    (self.node_selected + 1).min(self.tree.len().saturating_sub(1));
            }
            KeyCode::Left if self.screen == Screen::Builder => {
                self.focus = match self.focus {
                    Panel::Details => Panel::Tree,
                    _ => Panel::Inventory,
                };
            }
            KeyCode::Right if self.screen == Screen::Builder => {
                self.focus = match self.focus {
                    Panel::Inventory => Panel::Tree,
                    _ => Panel::Details,
                };
            }
            _ => return false,
        }
        true
    }

    fn enrollment(&self) -> Option<bool> {
        let status = self.status.as_ref()?;
        if status["registry_revision"].is_null() {
            return None;
        }
        Some(status["enrolled"].as_bool().unwrap_or(false))
    }

    fn strip(&self) -> Vec<Line<'static>> {
        let mut first = vec![match &self.preview {
            Some(p) => match p["collected_at"].as_u64() {
                Some(t) => format!("preview {} s ago", self.now.saturating_sub(t)),
                None => "preview time unknown".into(),
            },
            None => "no preview".into(),
        }];
        if let Some(r) = self
            .status
            .as_ref()
            .and_then(|s| s["registry_revision"].as_u64())
        {
            first.push(format!("registry revision {r}"));
        }
        if self.enrollment() == Some(false) {
            first.push("policy installed, not enrolled".into());
        }
        if let Some(p) = &self.preview
            && p["state"] == "conflicts"
        {
            let n = p["conflicts"].as_array().map_or(0, Vec::len);
            first.push(format!("{n} conflicts, see [c] Changes"));
        }
        first.push("legacy loop writes shaping".into());
        let mut lines = vec![Line::from(first.join("  |  "))];
        let mut second = vec!["edits apply after activation".to_string()];
        if self.pending > 0 {
            second.push("waiting for the service".to_string());
        }
        if let Some(e) = &self.error {
            second.push(e.clone());
        }
        lines.push(Line::from(second.join("  |  ")));
        lines
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let [strip, body] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        let mut lines = self.strip();
        if let Some(second) = lines.pop() {
            let style = if self.error.is_some() {
                Role::Error
            } else {
                Role::Secondary
            };
            f.render_widget(
                Paragraph::new(vec![lines.remove(0), second.style(p.role(style))])
                    .style(p.role(Role::Base)),
                strip,
            );
        }
        let not_enrolled = self
            .status
            .as_ref()
            .is_some_and(|s| s["registry_revision"].is_null());
        if not_enrolled {
            let text = vec![
                Line::from("Topology Manager is not enrolled."),
                Line::from(""),
                Line::from(
                    "sudo lqosync3 topology policy propose --out /root/topology-policy.json",
                ),
                Line::from("sudo lqosync3 topology check --policy /root/topology-policy.json"),
                Line::from("See docs/operations/TOPOLOGY_ENROLLMENT.md"),
            ];
            f.render_widget(Paragraph::new(text).style(p.role(Role::Base)), body);
            return;
        }
        match self.screen {
            Screen::Builder => builder::draw(self, f, body, p),
            Screen::Assignments => assignments::draw(self, f, body, p),
            Screen::Policies => policies::draw(self, f, body, p),
            Screen::Changes => changes::draw(self, f, body, p),
        }
        if let Some(picker) = &self.picker {
            picker::draw(self, picker, f, body, p);
        }
        if let Some(form) = &self.form {
            form.draw(f, body, p);
        }
        if let Some((_, name)) = &self.confirm_delete {
            let lines = vec![Line::from(format!("Delete plan {name}? [y/n]"))];
            plan_form::popup(f, body, lines, "Delete plan", p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::api::ApiAnswer;
    use crate::tui::theme::Palette;
    use ratatui::{Terminal, backend::TestBackend};
    use serde_json::{Value, json};

    pub(crate) fn render(v: &TopologyView, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        let palette = Palette::new(crate::config::Theme::Dark);
        t.draw(|f| v.draw(f, f.area(), &palette)).unwrap();
        let buf = t.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn ok(op: &str, v: Value) -> ApiAnswer {
        ApiAnswer {
            op: op.into(),
            result: Ok(v),
        }
    }
    fn ops(v: &mut TopologyView) -> Vec<String> {
        v.take_outbox().into_iter().map(|(op, _)| op).collect()
    }

    #[test]
    fn opening_asks_for_status_and_the_cached_preview() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        assert_eq!(ops(&mut v), vec!["status", "topology.preview.get"]);
        v.tick(true, 1_001);
        assert!(ops(&mut v).is_empty(), "open only once");
    }

    #[test]
    fn null_revision_shows_not_enrolled_with_runbook_commands() {
        let mut v = TopologyView::default();
        v.on_answer(ok(
            "status",
            json!({"registry_revision": null, "enrolled": false}),
        ));
        let text = render(&v, 120, 30);
        assert!(text.contains("not enrolled"), "{text}");
        assert!(text.contains("topology policy propose"), "{text}");
    }

    #[test]
    fn a_registry_without_ownership_shows_policy_installed() {
        let mut v = TopologyView::default();
        v.on_answer(ok(
            "status",
            json!({"registry_revision": 1, "enrolled": false}),
        ));
        assert!(render(&v, 120, 30).contains("policy installed, not enrolled"));
    }

    #[test]
    fn an_old_cached_preview_triggers_one_refresh() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        v.take_outbox();
        v.on_answer(ok(
            "topology.preview.get",
            json!({"collected_at": 500, "state": "ok"}),
        ));
        assert_eq!(ops(&mut v), vec!["topology.preview.refresh", "status"]);
        v.on_answer(ok(
            "topology.preview.get",
            json!({"collected_at": 500, "state": "ok"}),
        ));
        assert!(ops(&mut v).is_empty(), "one refresh while one is pending");
        let mut fresh = TopologyView::default();
        fresh.tick(true, 1_000);
        fresh.take_outbox();
        fresh.on_answer(ok(
            "topology.preview.get",
            json!({"collected_at": 900, "state": "ok"}),
        ));
        assert!(
            ops(&mut fresh).is_empty(),
            "a recent preview needs no refresh"
        );
    }

    #[test]
    fn the_strip_shows_preview_age_revision_and_legacy_writer() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        v.take_outbox();
        v.on_answer(ok(
            "status",
            json!({"registry_revision": 3, "enrolled": true}),
        ));
        v.on_answer(ok(
            "topology.preview.get",
            json!({"collected_at": 940, "state": "ok", "registry_revision": 3}),
        ));
        let text = render(&v, 120, 30);
        assert!(text.contains("60 s ago"), "{text}");
        assert!(text.contains("revision 3"), "{text}");
        assert!(text.contains("legacy loop writes shaping"), "{text}");
    }

    #[test]
    fn a_waiting_request_shows_waiting_for_the_service() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        assert!(render(&v, 120, 30).contains("waiting for the service"));
    }

    #[test]
    fn a_not_running_answer_keeps_the_tui_usable() {
        let mut v = TopologyView::default();
        v.on_answer(ApiAnswer {
            op: "status".into(),
            result: Err("lqosync3.service does not answer on /var/lib/lqosync3/api.sock".into()),
        });
        let text = render(&v, 120, 30);
        assert!(text.contains("does not answer"), "{text}");
        assert!(v.on_key(ratatui::crossterm::event::KeyCode::Char('r')));
        assert_eq!(ops(&mut v), vec!["topology.preview.refresh", "status"]);
    }

    #[test]
    fn a_failed_refresh_keeps_the_old_preview_and_shows_the_error() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        v.take_outbox();
        v.on_answer(ok(
            "topology.preview.get",
            json!({"collected_at": 990, "state": "ok"}),
        ));
        v.on_answer(ApiAnswer {
            op: "topology.preview.refresh".into(),
            result: Err("source read failed".into()),
        });
        assert_eq!(v.preview().unwrap()["collected_at"], json!(990));
        assert!(render(&v, 120, 30).contains("source read failed"));
    }

    #[test]
    fn view_keys_switch_the_four_views() {
        let mut v = TopologyView::default();
        for (c, s) in [
            ('a', Screen::Assignments),
            ('p', Screen::Policies),
            ('c', Screen::Changes),
            ('b', Screen::Builder),
        ] {
            assert!(v.on_key(ratatui::crossterm::event::KeyCode::Char(c)));
            assert_eq!(v.screen, s);
        }
    }

    #[test]
    fn reopening_the_route_asks_again_and_refresh_also_asks_status() {
        let mut v = TopologyView::default();
        v.tick(true, 1_000);
        v.take_outbox();
        v.tick(false, 1_100);
        v.tick(true, 1_200);
        assert_eq!(ops(&mut v), vec!["status", "topology.preview.get"]);
    }

    #[test]
    fn a_conflict_preview_shows_a_strip_hint() {
        let mut v = TopologyView::default();
        v.on_answer(ok(
            "status",
            json!({"registry_revision": 2, "enrolled": true}),
        ));
        v.on_answer(ok(
            "topology.preview.refresh",
            json!({"collected_at": 0, "state": "conflicts", "nodes": [], "subscribers": [], "inventory": [],
                   "rounded": [], "conflicts": [{"kind": "identity", "subject": "a", "detail": "x"},
                                                {"kind": "identity", "subject": "b", "detail": "x"}]}),
        ));
        assert!(render(&v, 120, 30).contains("2 conflicts, see [c] Changes"));
    }
}

mod builder;

mod assignments;

mod changes;
mod picker;
mod plan_form;
mod policies;
