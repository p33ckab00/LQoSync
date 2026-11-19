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
    /// The result of the last edit. A status or preview answer does not clear it.
    notice: Option<String>,
    picker: Option<picker::Picker>,
    form: Option<plan_form::PlanForm>,
    /// The plan that waits for a y/n answer before delete.
    confirm_delete: Option<(String, String)>,
    plan_selected: usize,
    /// The last `wan.discover` answer.
    wan: Option<Value>,
    /// The last `topology.tree` answer: every catalog node.
    tree_data: Option<Value>,
    inventory_selected: usize,
    /// The kept edit proposal that waits for Enter or Esc.
    proposal: Option<Value>,
    node_form: Option<node_form::NodeForm>,
    parent_picker: Option<parent_picker::ParentPicker>,
    discovering: bool,
    pub screen: Screen,
}

/// A selectable Builder Inventory line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InventoryItem {
    Source(String),
    Wan {
        router_id: String,
        endpoint_ref: String,
    },
}

/// A WAN candidate that gets its own Inventory line.
pub(crate) fn likely_wan(c: &Value) -> bool {
    c["suggested"] == true || !c["source_id"].is_null()
}

/// The first identity of a preview subscriber, or its ID when it has no identity list.
fn subscriber_key(s: &Value) -> Value {
    match &s["identities"][0] {
        Value::Null => s["id"].clone(),
        identity => identity.clone(),
    }
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
        self.send_args(op, json!({}));
    }

    fn send_args(&mut self, op: &str, args: Value) {
        self.outbox.push((op.to_string(), args));
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
        self.send("topology.tree");
    }

    /// True while a dialog of this route has all keys.
    pub fn capturing(&self) -> bool {
        self.picker.is_some()
            || self.form.is_some()
            || self.confirm_delete.is_some()
            || self.proposal.is_some()
            || self.node_form.is_some()
            || self.parent_picker.is_some()
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
        self.notice = None;
        self.outbox.push((op.to_string(), args));
        self.pending += 1;
    }

    fn propose(&mut self, edit: Value) {
        self.send_edit("topology.edit.propose", json!({"edit": edit}));
    }

    fn dialog_key(&mut self, code: KeyCode) {
        if let Some(form) = self.node_form.as_mut() {
            match form.key(code) {
                node_form::Step::Stay => {}
                node_form::Step::Cancel => self.node_form = None,
                node_form::Step::Send(edit) => {
                    self.node_form = None;
                    self.propose(edit);
                }
            }
            return;
        }
        if let Some(picker) = self.parent_picker.as_mut() {
            match picker.key(code) {
                parent_picker::Step::Stay => {}
                parent_picker::Step::Cancel => self.parent_picker = None,
                parent_picker::Step::Send(edit) => {
                    self.parent_picker = None;
                    self.propose(edit);
                }
            }
            return;
        }
        if let Some(p) = &self.proposal {
            let args = json!({"proposal_id": p["proposal_id"]});
            match code {
                KeyCode::Enter => {
                    self.proposal = None;
                    self.notice = None;
                    self.send_args("topology.edit.confirm", args);
                }
                KeyCode::Esc => {
                    self.proposal = None;
                    self.send_args("topology.edit.cancel", args);
                }
                _ => {}
            }
            return;
        }
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
            // A new account needs a preview of the current revision, so wait for the refresh.
            KeyCode::Enter if self.refreshing => {
                self.notice = Some("wait for the preview refresh, then try again".into());
            }
            KeyCode::Enter => {
                let selected = picker.selected;
                let identity = Some(picker.identity.clone()).filter(|i| !i.is_null());
                self.picker = None;
                let plan_id = picker::entries(self.policies.as_ref())
                    .swap_remove(selected)
                    .0;
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

    /// The Tree rows: catalog nodes when the tree answer is present, else preview nodes.
    fn tree_nodes(&self) -> &[Value] {
        match self.tree_data.as_ref().and_then(|t| t["nodes"].as_array()) {
            Some(nodes) => nodes,
            None => self.nodes(),
        }
    }

    /// The preview row of a node, when the preview has one.
    pub(crate) fn preview_node(&self, id: &str) -> Option<&Value> {
        self.nodes().iter().find(|n| n["id"].as_str() == Some(id))
    }

    fn selected_node(&self) -> Option<&Value> {
        let &(_, i) = self.tree.get(self.node_selected)?;
        self.tree_nodes().get(i)
    }

    /// Rebuild the Tree order and keep the selected node when it still exists.
    fn rebuild_tree(&mut self, selected: Option<String>) {
        self.tree = builder::index(self.tree_nodes());
        self.node_selected = selected
            .and_then(|id| {
                self.tree
                    .iter()
                    .position(|&(_, i)| self.tree_nodes()[i]["id"].as_str() == Some(id.as_str()))
            })
            .unwrap_or(0)
            .min(self.tree.len().saturating_sub(1));
    }

    fn selected_node_id(&self) -> Option<String> {
        self.selected_node()
            .and_then(|n| n["id"].as_str())
            .map(str::to_string)
    }

    pub(crate) fn inventory_items(&self) -> Vec<InventoryItem> {
        let mut items: Vec<InventoryItem> = self
            .preview()
            .and_then(|p| p["inventory"].as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| s["id"].as_str().map(|id| InventoryItem::Source(id.into())))
            .collect();
        let routers = self.wan.as_ref().and_then(|w| w["routers"].as_array());
        for r in routers.into_iter().flatten().filter(|r| r["state"] == "ok") {
            for c in r["candidates"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|c| likely_wan(c))
            {
                items.push(InventoryItem::Wan {
                    router_id: r["router_id"].as_str().unwrap_or_default().into(),
                    endpoint_ref: c["name"].as_str().unwrap_or_default().into(),
                });
            }
        }
        items
    }

    pub(crate) fn inventory_item(&self) -> Option<InventoryItem> {
        self.inventory_items().get(self.inventory_selected).cloned()
    }

    /// A parent picker over the catalog nodes in tree order.
    fn pick(&mut self, purpose: parent_picker::Purpose) {
        let all = self.tree_nodes();
        let ordered: Vec<&Value> = self.tree.iter().map(|&(_, i)| &all[i]).collect();
        self.parent_picker = Some(parent_picker::ParentPicker::new(&ordered, all, purpose));
    }

    /// The selected catalog node. Edits need the `topology.tree` answer.
    fn selected_catalog_node(&mut self) -> Option<Value> {
        if self.tree_data.is_none() {
            self.error = Some("the tree is not loaded, press r".into());
            return None;
        }
        self.selected_node().cloned()
    }

    fn parent_id(&self, id: &str) -> Option<String> {
        self.tree_nodes()
            .iter()
            .find(|n| n["id"].as_str() == Some(id))
            .and_then(|n| n["parent"].as_str())
            .map(str::to_string)
    }

    /// Builder edit keys. True when the key started an action.
    fn builder_key(&mut self, code: KeyCode) -> bool {
        if code == KeyCode::Enter && self.focus == Panel::Inventory {
            match self.inventory_item() {
                Some(InventoryItem::Source(id)) => {
                    if self.tree_data.is_some() {
                        self.pick(parent_picker::Purpose::Instance(id));
                    } else {
                        self.error = Some("the tree is not loaded, press r".into());
                    }
                }
                Some(InventoryItem::Wan {
                    router_id,
                    endpoint_ref,
                }) => self.node_form = Some(node_form::NodeForm::wan(&router_id, &endpoint_ref)),
                None => {}
            }
            return true;
        }
        let action = match code {
            KeyCode::Char(c @ ('n' | 'e' | 'm' | '<' | '>' | 'x')) => c,
            _ => return false,
        };
        let Some(node) = self.selected_catalog_node() else {
            return true;
        };
        let id = node["id"].as_str().unwrap_or_default().to_string();
        match action {
            'n' => {
                let parent = if node["kind"] == "group" {
                    node["parent"].as_str().map(str::to_string)
                } else {
                    Some(id)
                };
                self.node_form = Some(node_form::NodeForm::intermediate(parent));
            }
            'e' => self.node_form = Some(node_form::NodeForm::update(&node)),
            'm' if node["kind"] == "wan" => self.notice = Some("a WAN node stays a root".into()),
            'm' => self.pick(parent_picker::Purpose::Move(id)),
            '<' => match node["parent"].as_str() {
                None => self.notice = Some("the node is at the root".into()),
                Some(parent) => {
                    let grandparent = self.parent_id(parent);
                    self.propose(
                        json!({"kind": "node.move", "node_id": id, "parent": grandparent}),
                    );
                }
            },
            '>' => self.pick(parent_picker::Purpose::Demote(id)),
            _ => self.propose(json!({"kind": "node.remove", "node_id": id})),
        }
        true
    }

    fn set_preview(&mut self, v: Value) {
        let selected = self.selected_node_id();
        // A temporary ID can move to another account after an edit. The identity does not.
        let subscriber = self.selected_subscriber().map(subscriber_key);
        let old_row = self.sub_selected;
        self.preview = Some(v);
        self.rebuild_tree(selected);
        // Keep the same subscriber, or the nearest row when it is gone.
        self.sub_selected = subscriber
            .and_then(|k| {
                self.subscribers()
                    .iter()
                    .position(|s| subscriber_key(s) == k)
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
            self.send("topology.tree");
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
            ("subscriber.plan" | "plan.create" | "plan.update" | "plan.delete", Ok(v)) => {
                self.error = None;
                // The next edit uses the new revision before the status answer arrives.
                if let (Some(status), Some(revision)) =
                    (self.status.as_mut(), v["revision"].as_u64())
                {
                    status["registry_revision"] = json!(revision);
                }
                self.reload();
            }
            ("subscriber.plan" | "plan.create" | "plan.update" | "plan.delete", Err(e))
                if is_conflict(&e) =>
            {
                self.notice = Some("changed by another operator, try again".into());
                self.reload();
            }
            ("topology.tree", Ok(v)) => {
                let selected = self.selected_node_id();
                self.tree_data = Some(v);
                self.rebuild_tree(selected);
            }
            ("topology.edit.propose", Ok(v)) => {
                self.error = None;
                self.proposal = Some(v);
            }
            ("topology.edit.confirm", Ok(v)) => {
                self.error = None;
                if let (Some(status), Some(revision)) =
                    (self.status.as_mut(), v["revision"].as_u64())
                {
                    status["registry_revision"] = json!(revision);
                }
                self.reload();
            }
            ("topology.edit.propose" | "topology.edit.confirm", Err(e)) if is_conflict(&e) => {
                self.notice = Some("changed by another operator, try again".into());
                self.reload();
            }
            ("topology.edit.cancel", _) => {}
            ("wan.discover", result) => {
                self.discovering = false;
                match result {
                    Ok(v) => self.wan = Some(v),
                    Err(e) => self.error = Some(e),
                }
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
        if self.screen == Screen::Builder && self.builder_key(code) {
            return true;
        }
        match code {
            KeyCode::Enter
                if self.screen == Screen::Assignments && self.selected_subscriber().is_some() =>
            {
                let identity = self
                    .selected_subscriber()
                    .map_or(Value::Null, |s| s["identities"][0].clone());
                self.picker = Some(picker::Picker {
                    selected: 0,
                    identity,
                });
                if self.policies.is_none() {
                    self.send("topology.policies");
                }
            }
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('w') if self.screen == Screen::Builder => {
                if !self.discovering {
                    self.discovering = true;
                    self.send("wan.discover");
                }
            }
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
            KeyCode::Up if self.screen == Screen::Builder && self.focus == Panel::Inventory => {
                self.inventory_selected = self.inventory_selected.saturating_sub(1);
            }
            KeyCode::Down if self.screen == Screen::Builder && self.focus == Panel::Inventory => {
                self.inventory_selected = (self.inventory_selected + 1)
                    .min(self.inventory_items().len().saturating_sub(1));
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
        if let Some(n) = &self.notice {
            second.push(n.clone());
        }
        lines.push(Line::from(second.join("  |  ")));
        lines
    }

    pub fn draw(&self, f: &mut Frame, area: Rect, p: &Palette) {
        let [strip, body] =
            Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).areas(area);
        let mut lines = self.strip();
        if let Some(second) = lines.pop() {
            let style = if self.error.is_some() || self.notice.is_some() {
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
        if let Some(proposal) = &self.proposal {
            proposal::draw(proposal, self.tree_nodes(), f, body, p);
        }
        if let Some(form) = &self.node_form {
            form.draw(f, body, p);
        }
        if let Some(picker) = &self.parent_picker {
            picker.draw(f, body, p);
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
        assert_eq!(
            ops(&mut v),
            vec!["status", "topology.preview.get", "topology.tree"]
        );
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
        assert_eq!(
            ops(&mut v),
            vec!["status", "topology.preview.get", "topology.tree"]
        );
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
mod node_form;
mod parent_picker;
mod picker;
mod plan_form;
mod policies;
mod proposal;
