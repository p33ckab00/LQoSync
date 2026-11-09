//! Approved Settings composition. The existing editor remains the only apply authority.
use super::form::RouterForm;
use super::*;
use crate::tui::design::{self, Scroll};
use ratatui::crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

const CATEGORIES: [(&str, &str); 14] = [
    ("runtime", "Runtime"),
    ("router_access", "Router access"),
    ("safety", "Safety"),
    ("libreqos", "LibreQoS"),
    ("network", "Network"),
    ("nodes", "Nodes"),
    ("circuits", "Circuits"),
    ("speed", "Speed"),
    ("bandwidth_defaults", "Bandwidth defaults"),
    ("backstop_caps", "Backstop caps"),
    ("telegram", "Telegram"),
    ("tui", "Interface"),
    ("exclude", "Exclusions"),
    ("routers", "Routers"),
];

#[derive(Default)]
pub(super) struct SettingsUi {
    pub form: Option<RouterForm>,
    pub diff_dirty: bool,
    loaded: Option<(String, Draft)>,
    diff: Vec<Line<'static>>,
    categories: Scroll,
    keys: Scroll,
    review: Scroll,
    help: Scroll,
    routers: Scroll,
    body: Scroll,
    popup: Scroll,
    category_targets: Vec<(Rect, usize)>,
    key_targets: Vec<(Rect, usize)>,
    router_targets: Vec<(Rect, usize)>,
    buttons: [Rect; 10],
    form_targets: [Rect; 15],
    popup_yes: Rect,
    popup_no: Rect,
    popup_rows: Vec<(Rect, usize)>,
    popup_ops: [Rect; 4],
    router_list: bool,
}
impl SettingsView {
    pub fn clear_approved_areas(&mut self) {
        self.approved.category_targets.clear();
        self.approved.key_targets.clear();
        self.approved.router_targets.clear();
        self.approved.buttons = [Rect::default(); 10];
        self.approved.form_targets = [Rect::default(); 15];
        self.approved.popup_yes = Rect::default();
        self.approved.popup_no = Rect::default();
        self.approved.popup_rows.clear();
        self.approved.popup_ops = [Rect::default(); 4];
        self.approved.categories.clear();
        self.approved.keys.clear();
        self.approved.review.clear();
        self.approved.help.clear();
        self.approved.routers.clear();
        self.approved.body.clear();
        self.approved.popup.clear();
    }
    fn category(&self) -> usize {
        if self.approved.router_list || self.approved.form.is_some() {
            return 13;
        }
        let Some(g) = self.groups.get(self.group) else {
            return 0;
        };
        if g.router.is_some() || g.title.starts_with("router_defaults") {
            return 13;
        }
        CATEGORIES
            .iter()
            .position(|(key, _)| g.title.split('.').next() == Some(key))
            .unwrap_or(0)
    }
    fn category_groups(&self, category: usize) -> Vec<usize> {
        self.groups
            .iter()
            .enumerate()
            .filter(|(_, g)| {
                if category == 13 {
                    g.router.is_some() || g.title.starts_with("router_defaults")
                } else {
                    g.router.is_none() && g.title.split('.').next() == Some(CATEGORIES[category].0)
                }
            })
            .map(|(i, _)| i)
            .collect()
    }
    fn select_category(&mut self, category: usize) {
        self.approved.router_list = category == 13;
        if let Some(&g) = self.category_groups(category).first() {
            self.group = g;
            self.key = 0;
            self.focus = 0;
        }
        self.approved.keys.offset = 0;
    }
    fn next_section(&mut self) {
        let groups = self.category_groups(self.category());
        if let Some(i) = groups.iter().position(|&i| i == self.group) {
            self.group = groups[(i + 1) % groups.len()];
            self.key = 0;
            self.approved.keys.offset = 0;
        }
    }
    fn open_form(&mut self) {
        if self.blocked() {
            return;
        }
        match self.draft.as_ref().map(|d| RouterForm::new(&d.text())) {
            Some(Ok(form)) => {
                self.approved.form = Some(form);
                self.approved.router_list = true;
            }
            Some(Err(e)) => self.message = Some(e),
            _ => {}
        }
    }
    fn commit_form(&mut self) {
        if self.blocked() {
            return;
        }
        let Some(form) = self.approved.form.as_ref() else {
            return;
        };
        let result = (|| -> Result<_, String> {
            let r = form.config()?;
            let text = self.draft.as_ref().ok_or("no draft")?.text();
            let mut trial = Draft::new(&text).map_err(|e| e.to_string())?;
            trial.add_router_config(&r).map_err(|e| e.to_string())?;
            validate::parse(&trial.text())?;
            Ok(r)
        })();
        let r = match result {
            Ok(r) => r,
            Err(e) => {
                self.message = Some(e);
                return;
            }
        };
        let result = self.draft.as_mut().map(|d| d.add_router_config(&r));
        if let Some(Ok(i)) = result {
            let form = self.approved.form.take().expect("form exists");
            self.add_pending(PathBuf::from(r.password_file), form.password);
            self.changed_now();
            self.select(&format!("routers.{i}.name"));
            self.approved.router_list = false;
            self.message =
                Some("Router added to draft only. Validate & preview before Save & apply".into());
        } else {
            self.message = Some("Cannot add router to draft".into());
        }
    }
    fn form_activate(&mut self, index: usize) {
        if index == 13 {
            self.commit_form();
            return;
        }
        if index == 14 {
            self.approved.form = None;
            self.message = Some("Form cancelled; no file changed".into());
            return;
        }
        if let Some(f) = self.approved.form.as_mut() {
            f.focus = index;
            match index {
                6..=10 => {
                    f.flags[index - 6] = !f.flags[index - 6];
                    f.editing = false;
                }
                12 => {
                    f.advanced = !f.advanced;
                    f.editing = false;
                }
                _ => f.editing = true,
            }
        }
    }
    fn form_key(&mut self, k: KeyEvent) {
        if k.code == KeyCode::Esc
            || k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('c')
        {
            self.approved.form = None;
            self.message = Some("Form cancelled; no file changed".into());
            return;
        }
        let Some(form) = self.approved.form.as_mut() else {
            return;
        };
        match k.code {
            KeyCode::Tab => form.next(false),
            KeyCode::BackTab => form.next(true),
            KeyCode::Enter if form.editing => {
                form.editing = false;
                form.next(false);
            }
            KeyCode::Enter | KeyCode::Char(' ') if !form.editing => {
                let i = form.focus;
                self.form_activate(i);
            }
            _ => form.edit_key(k),
        }
    }
    pub fn approved_key(&mut self, k: KeyEvent) -> SAction {
        if self.approved.form.is_some() {
            self.form_key(k);
            return SAction::None;
        }
        if self.capturing() {
            return self.on_key(k);
        }
        match k.code {
            KeyCode::Char('+') => {
                self.open_form();
                SAction::None
            }
            KeyCode::Char('g') => {
                self.next_section();
                SAction::None
            }
            KeyCode::Up if self.focus == 0 => {
                let c = self.category().saturating_sub(1);
                self.select_category(c);
                SAction::None
            }
            KeyCode::Down if self.focus == 0 => {
                let c = (self.category() + 1).min(13);
                self.select_category(c);
                SAction::None
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = 1 - self.focus.min(1);
                SAction::None
            }
            _ => self.on_key(k),
        }
    }
    fn discard_draft(&mut self) {
        if self.blocked() {
            return;
        }
        self.approved.form = None;
        match Draft::new(&self.raw) {
            Ok(d) => {
                self.draft = Some(d);
                self.secrets.clear();
                self.secrets_rev += 1;
                self.shown = None;
                self.running = None;
                self.after_change();
                self.message = Some("Draft discarded; no file changed".into());
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }
    pub fn approved_mouse(&mut self, m: MouseEvent) -> SAction {
        if self.modal.is_some()
            || !matches!(
                self.input,
                Input::None
                    | Input::Field {
                        list_add: false,
                        ..
                    }
            )
        {
            if m.kind == MouseEventKind::Down(MouseButton::Left) {
                if let Some(i) = self
                    .approved
                    .popup_ops
                    .iter()
                    .position(|r| r.contains((m.column, m.row).into()))
                {
                    return self.on_key(KeyEvent::new(
                        KeyCode::Char(['a', 'd', 'K', 'J'][i]),
                        m.modifiers,
                    ));
                }
                if self.approved.popup_yes.contains((m.column, m.row).into()) {
                    return self.on_key(KeyEvent::new(
                        if self.modal.is_some() {
                            KeyCode::Char('y')
                        } else {
                            KeyCode::Enter
                        },
                        m.modifiers,
                    ));
                }
                if self.approved.popup_no.contains((m.column, m.row).into()) {
                    return self.on_key(KeyEvent::new(KeyCode::Esc, m.modifiers));
                }
                if let Some((_, index)) = self
                    .approved
                    .popup_rows
                    .iter()
                    .find(|(r, _)| r.contains((m.column, m.row).into()))
                {
                    match &mut self.input {
                        Input::Dropdown { sel, .. } | Input::List { sel, .. } => *sel = *index,
                        _ => {}
                    }
                    return self.on_key(KeyEvent::new(KeyCode::Enter, m.modifiers));
                }
            }
            if self.approved.popup.mouse(m) {
                match &mut self.input {
                    Input::Dropdown { sel, .. } | Input::List { sel, .. } => {
                        *sel = self.approved.popup.offset
                    }
                    _ => {}
                }
            }
            return SAction::None;
        }
        if self.approved.form.is_some() {
            if m.kind == MouseEventKind::Down(MouseButton::Left)
                && let Some(i) = self
                    .approved
                    .form_targets
                    .iter()
                    .position(|r| r.contains((m.column, m.row).into()))
            {
                self.form_activate(i);
            }
            self.approved.body.mouse(m);
            return SAction::None;
        }
        if m.kind == MouseEventKind::Down(MouseButton::Left) {
            if matches!(self.input, Input::Field { .. }) {
                let point = (m.column, m.row).into();
                let owns = self
                    .approved
                    .category_targets
                    .iter()
                    .chain(self.approved.key_targets.iter())
                    .chain(self.approved.router_targets.iter())
                    .any(|(r, _)| r.contains(point))
                    || self.approved.buttons.iter().any(|r| r.contains(point));
                if !owns {
                    return SAction::None;
                }
                if self
                    .approved
                    .key_targets
                    .iter()
                    .any(|(r, i)| *i == self.key && r.contains(point))
                {
                    return SAction::None;
                }
                self.on_key(KeyEvent::new(KeyCode::Enter, m.modifiers));
                if !matches!(self.input, Input::None) {
                    return SAction::None;
                }
            }
            if let Some((_, category)) = self
                .approved
                .category_targets
                .iter()
                .find(|(r, _)| r.contains((m.column, m.row).into()))
            {
                self.select_category(*category);
                return SAction::None;
            }
            if let Some((_, i)) = self
                .approved
                .router_targets
                .iter()
                .find(|(r, _)| r.contains((m.column, m.row).into()))
            {
                let i = *i;
                self.select(&format!("routers.{i}.name"));
                self.approved.router_list = false;
                return SAction::None;
            }
            if let Some((_, key)) = self
                .approved
                .key_targets
                .iter()
                .find(|(r, _)| r.contains((m.column, m.row).into()))
            {
                self.key = *key;
                self.focus = 1;
                return self.on_key(KeyEvent::new(KeyCode::Enter, m.modifiers));
            }
            if let Some(i) = self
                .approved
                .buttons
                .iter()
                .position(|r| r.contains((m.column, m.row).into()))
            {
                match i {
                    0 => return self.on_key(KeyEvent::new(KeyCode::Char('v'), m.modifiers)),
                    1 => return self.on_key(KeyEvent::new(KeyCode::Char('A'), m.modifiers)),
                    2 => self.discard_draft(),
                    3 => self.open_form(),
                    4 => self.next_section(),
                    5 => return self.on_key(KeyEvent::new(KeyCode::Char('-'), m.modifiers)),
                    6 => {
                        self.focus = 1;
                        return self.on_key(KeyEvent::new(KeyCode::Char('P'), m.modifiers));
                    }
                    7 => {
                        self.focus = 1;
                        return self.on_key(KeyEvent::new(KeyCode::Char('r'), m.modifiers));
                    }
                    8 => return self.on_key(KeyEvent::new(KeyCode::Char('u'), m.modifiers)),
                    9 => return self.on_key(KeyEvent::new(KeyCode::Char('D'), m.modifiers)),
                    _ => {}
                }
                return SAction::None;
            }
        }
        if self.approved.keys.mouse(m) {
            let count = self
                .groups
                .get(self.group)
                .map_or(0, |g| g.keys.len() + g.extra.len());
            self.key = self.approved.keys.offset.min(count.saturating_sub(1));
            self.focus = 1;
        } else if !self.approved.categories.mouse(m)
            && !self.approved.review.mouse(m)
            && !self.approved.help.mouse(m)
            && !self.approved.routers.mouse(m)
        {
            self.approved.body.mouse(m);
        }
        SAction::None
    }
    fn refresh_approved(&mut self) {
        let Some(d) = self.draft.as_ref() else {
            return;
        };
        if self
            .approved
            .loaded
            .as_ref()
            .is_none_or(|(key, _)| key != &d.start_digest)
        {
            self.approved.loaded = Draft::new(d.original())
                .ok()
                .map(|v| (d.start_digest.clone(), v));
            self.approved.diff_dirty = true;
        }
        if self.approved.diff_dirty {
            self.approved.diff = Vec::new();
            if let Some((_, original)) = &self.approved.loaded {
                for text in &self.changed {
                    let p = key_path(text);
                    let was = original
                        .effective(&p)
                        .map_or("(not set)".into(), |(v, _)| show(&v));
                    let now = d
                        .effective(&p)
                        .map_or("(not set)".into(), |(v, _)| show(&v));
                    self.approved
                        .diff
                        .push(Line::raw(format!("- {text} = {was}")));
                    self.approved
                        .diff
                        .push(Line::raw(format!("+ {text} = {now}")));
                }
            }
            self.approved.diff_dirty = false;
        }
    }
    pub fn draw_approved(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        self.refresh_approved();
        let height = area.height.max(35);
        self.approved.body.offset = self
            .approved
            .body
            .offset
            .min(height.saturating_sub(area.height) as usize);
        if height > area.height {
            let mut t = ratatui::Terminal::new(ratatui::backend::TestBackend::new(
                area.width.saturating_sub(1),
                height,
            ))
            .expect("in-memory terminal");
            t.draw(|v| self.draw_body(v, v.area(), p))
                .expect("in-memory draw");
            let b = t.backend().buffer();
            for y in 0..area.height {
                for x in 0..area.width.saturating_sub(1) {
                    f.buffer_mut()[(area.x + x, area.y + y)] =
                        b[(x, y + self.approved.body.offset as u16)].clone();
                }
            }
            let map = |r: Rect| design::viewport_rect(r, area, self.approved.body.offset);
            for (r, _) in self
                .approved
                .category_targets
                .iter_mut()
                .chain(self.approved.key_targets.iter_mut())
                .chain(self.approved.router_targets.iter_mut())
            {
                *r = map(*r);
            }
            self.approved.buttons = self.approved.buttons.map(map);
            self.approved.form_targets = self.approved.form_targets.map(map);
            self.approved.categories.area = map(self.approved.categories.area);
            self.approved.keys.area = map(self.approved.keys.area);
            self.approved.review.area = map(self.approved.review.area);
            self.approved.help.area = map(self.approved.help.area);
            self.approved.routers.area = map(self.approved.routers.area);
        } else {
            self.draw_body(f, area, p);
        }
        self.approved.body.draw(f, area, height as usize, p);
        self.draw_approved_popup(f, area, p);
    }
    fn draw_body(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        f.render_widget(Block::default().style(design::surface(p)), area);
        let [head, draft, body, review, status] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(20),
            Constraint::Length(7),
            Constraint::Length(3),
        ])
        .areas(area);
        f.render_widget(
            Paragraph::new(format!("Configuration  {}", self.path.display()))
                .block(design::panel("", p)),
            head,
        );
        let current = self.current_preview().is_some();
        let ready = self
            .current_preview()
            .is_some_and(|s| !matches!(s.result, Preview::Failed(_)));
        f.render_widget(
            Paragraph::new(format!(
                "Draft: {} changes | Not saved | {}{}",
                self.changed.len(),
                if current {
                    "Preview current"
                } else {
                    "Preview required"
                },
                if self.approved.form.is_some() {
                    " | Add Router form: UNSAVED"
                } else if matches!(self.input, Input::Field { .. }) {
                    " | Field edit not committed"
                } else {
                    ""
                }
            ))
            .style(design::role(p, Role::Warning)),
            draft,
        );
        let [groups, content] =
            Layout::horizontal([Constraint::Percentage(16), Constraint::Min(0)]).areas(body);
        let block = design::panel("Settings groups", p);
        let inner = block.inner(groups);
        f.render_widget(block, groups);
        self.approved.categories.offset = self
            .approved
            .categories
            .offset
            .min(CATEGORIES.len().saturating_sub(inner.height as usize));
        let selected = self.category();
        if selected < self.approved.categories.offset {
            self.approved.categories.offset = selected;
        } else if selected >= self.approved.categories.offset + inner.height as usize
            && inner.height > 0
        {
            self.approved.categories.offset = selected + 1 - inner.height as usize;
        }
        for (y, (_, label)) in CATEGORIES
            .iter()
            .enumerate()
            .skip(self.approved.categories.offset)
            .take(inner.height as usize)
        {
            let r = Rect::new(
                inner.x,
                inner.y + (y - self.approved.categories.offset) as u16,
                inner.width,
                1,
            );
            self.approved.category_targets.push((r, y));
            f.render_widget(
                Paragraph::new(*label).style(if y == selected {
                    design::selected(p)
                } else {
                    design::surface(p)
                }),
                r,
            );
        }
        self.approved.categories.draw(f, inner, CATEGORIES.len(), p);
        if self.approved.router_list || self.approved.form.is_some() {
            let [routers, fields] =
                Layout::horizontal([Constraint::Percentage(28), Constraint::Min(0)]).areas(content);
            self.draw_router_list(f, routers, p);
            if self.approved.form.is_some() {
                self.draw_router_form(f, fields, p);
            } else if self
                .groups
                .get(self.group)
                .is_some_and(|g| g.router.is_some() || g.title.starts_with("router_defaults"))
            {
                self.draw_values(f, fields, p);
            } else {
                f.render_widget(
                    Paragraph::new("Select a router, or choose + New router")
                        .block(design::panel("Router settings", p)),
                    fields,
                );
            }
        } else {
            let [values, changes] =
                Layout::horizontal([Constraint::Percentage(64), Constraint::Min(0)]).areas(content);
            self.draw_values(f, values, p);
            self.draw_changes(f, changes, p);
        }
        let block = design::panel("Review and apply", p);
        let inner = block.inner(review);
        f.render_widget(block, review);
        let validation = if current {
            if ready { "VALID" } else { "FAILED" }
        } else {
            "NOT RUN / draft changed"
        };
        f.render_widget(
            Paragraph::new(format!(
                "Validation {validation} | Preview {} | {} draft changes",
                if current { "CURRENT" } else { "NOT CURRENT" },
                self.changed.len()
            )),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        let width = (inner.width / 3).max(1);
        for (i, label) in [
            "[v Validate & preview]",
            "[A Save & apply]",
            "[Discard draft]",
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(inner.x + i as u16 * width, inner.y + 1, width, 1);
            self.approved.buttons[i] = r;
            let enabled = !self.read_only
                && self.draft.is_some()
                && self.dirty()
                && self.approved.form.is_none()
                && (i != 1 || ready);
            f.render_widget(
                Paragraph::new(label).style(if enabled {
                    design::accent(p)
                } else {
                    design::role(p, Role::Secondary)
                }),
                r,
            );
        }
        f.render_widget(
            Paragraph::new("Review -> confirm -> backup -> save -> reload -> verify"),
            Rect::new(inner.x, inner.y + 2, inner.width, 1),
        );
        let output = self
            .current_preview()
            .map(|s| match &s.result {
                Preview::Ran(_) => "Output preview complete",
                Preview::Skipped(why) => why.as_str(),
                Preview::Failed(why) => why.as_str(),
            })
            .unwrap_or("A current exact-draft preview is required before Save & apply");
        f.render_widget(
            Paragraph::new(output),
            Rect::new(inner.x, inner.y + 3, inner.width, 1),
        );
        let stage = match &self.stage {
            Stage::Edit => "Draft only; no file changed",
            Stage::Verifying { .. } => "Waiting for service result verification",
            Stage::Result(text) => text.as_str(),
        };
        f.render_widget(
            Paragraph::new(format!(
                "{} | {stage} | {}",
                if self.read_only {
                    "READ-ONLY"
                } else {
                    "Write access available"
                },
                self.message.as_deref().unwrap_or("")
            ))
            .block(design::panel("", p)),
            status,
        );
    }
    fn draw_values(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        let title = self
            .groups
            .get(self.group)
            .map_or("Values", |g| g.title.as_str());
        let block = design::panel(format!("{title} values"), p);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let [section, header, rows, help] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(4),
        ])
        .areas(inner);
        self.approved.buttons[4] = section;
        let third = section.width / 3;
        for (id, index, label) in [
            (4, 0, "[g Next section]"),
            (6, 1, "[P Set secret]"),
            (5, 2, "[- Remove]"),
        ] {
            let r = Rect::new(section.x + index * third, section.y, third, 1);
            self.approved.buttons[id] = r;
            f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
        }
        let widths = Layout::horizontal([
            Constraint::Percentage(34),
            Constraint::Percentage(23),
            Constraint::Percentage(27),
            Constraint::Min(0),
        ])
        .split(header);
        for (i, label) in ["Key", "Loaded", "Draft", "Unit"].into_iter().enumerate() {
            f.render_widget(Paragraph::new(label).style(design::accent(p)), widths[i]);
        }
        let Some(group) = self.groups.get(self.group) else {
            return;
        };
        let count = group.keys.len() + group.extra.len();
        let h = rows.height as usize;
        if self.key < self.approved.keys.offset {
            self.approved.keys.offset = self.key;
        } else if h > 0 && self.key >= self.approved.keys.offset + h {
            self.approved.keys.offset = self.key + 1 - h;
        }
        for index in self.approved.keys.offset..(self.approved.keys.offset + h).min(count) {
            let y = rows.y + (index - self.approved.keys.offset) as u16;
            let r = Rect::new(rows.x, y, rows.width, 1);
            self.approved.key_targets.push((r, index));
            let style = if self.focus == 1 && self.key == index {
                design::selected(p)
            } else {
                design::surface(p)
            };
            if let Some((path, meta)) = group.keys.get(index) {
                let at = path_text(path);
                let was = self
                    .approved
                    .loaded
                    .as_ref()
                    .and_then(|(_, d)| d.effective(path))
                    .map_or("(not set)".into(), |(v, _)| show(&v));
                let mut draft = self
                    .draft
                    .as_ref()
                    .and_then(|d| d.effective(path))
                    .map_or("(not set)".into(), |(v, _)| show(&v));
                if let Input::Field {
                    path: p,
                    buf,
                    list_add: false,
                    ..
                } = &self.input
                    && p == path
                {
                    draft = format!("{buf}_");
                }
                let unit = if at.contains("mbps") {
                    "Mbps"
                } else if at.ends_with("_ms") {
                    "ms"
                } else if at.contains("interval")
                    || at.ends_with("timeout")
                    || at.ends_with("_age")
                    || at.ends_with("grace")
                {
                    "seconds"
                } else if at.ends_with("keep_backups") {
                    "backups"
                } else {
                    "--"
                };
                let cells = Layout::horizontal([
                    Constraint::Percentage(34),
                    Constraint::Percentage(23),
                    Constraint::Percentage(27),
                    Constraint::Min(0),
                ])
                .split(r);
                let name = meta.path.rsplit('.').next().unwrap_or(&meta.path);
                for (i, text) in [name.to_owned(), was, draft, unit.into()]
                    .into_iter()
                    .enumerate()
                {
                    f.render_widget(
                        Paragraph::new(text).style(
                            if self.changed.contains(&at) && !(self.focus == 1 && self.key == index)
                            {
                                design::role(p, Role::Warning)
                            } else {
                                style
                            },
                        ),
                        cells[i],
                    );
                }
            } else if let Some(extra) = group.extra.get(index - group.keys.len()) {
                let text = match extra {
                    Extra::Note(s) => s.clone(),
                    Extra::Lease(l) => format!("{} {} {}", l.mac, l.ip, l.host),
                    Extra::Client { text, .. } => text.clone(),
                };
                f.render_widget(Paragraph::new(text).style(style), r);
            }
        }
        self.approved.keys.draw(f, rows, count, p);
        let text = self
            .current()
            .map(|(path, m)| {
                format!(
                    "{}\n{}\nEnter edit | Space toggle | r Reset | u Undo",
                    path_text(path),
                    m.help
                )
            })
            .unwrap_or_else(|| "Select a value or router record".into());
        let help_area = Rect::new(help.x, help.y, help.width, help.height.saturating_sub(1));
        // Metadata help is plain text. Break long paths as well as words so every line remains reachable.
        let width = help_area.width.max(1) as usize;
        let mut help_lines = Vec::new();
        for source in text.lines() {
            let mut line = String::new();
            for word in source.split_whitespace() {
                if !line.is_empty() && Line::from(format!("{line} {word}")).width() > width {
                    help_lines.push(line);
                    line = String::new();
                }
                if !line.is_empty() {
                    line.push(' ');
                }
                for ch in word.chars() {
                    if !line.is_empty() && Line::from(format!("{line}{ch}")).width() > width {
                        help_lines.push(line);
                        line = String::new();
                    }
                    line.push(ch);
                }
            }
            help_lines.push(line);
        }
        let total = help_lines.len();
        let paragraph = Paragraph::new(help_lines.join("\n"));
        self.approved.help.offset = self
            .approved
            .help
            .offset
            .min(total.saturating_sub(help_area.height as usize));
        f.render_widget(
            paragraph.scroll((self.approved.help.offset as u16, 0)),
            help_area,
        );
        self.approved.help.draw(f, help_area, total, p);
        let row = help.bottom().saturating_sub(1);
        self.approved.buttons[7] = Rect::new(help.x, row, help.width / 2, 1);
        self.approved.buttons[8] = Rect::new(help.x + help.width / 2, row, help.width / 2, 1);
        f.render_widget(
            Paragraph::new("[r Reset selected]").style(design::accent(p)),
            self.approved.buttons[7],
        );
        f.render_widget(
            Paragraph::new("[u Undo]").style(design::accent(p)),
            self.approved.buttons[8],
        );
    }
    fn draw_changes(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        let block = design::panel("Review changes", p);
        let full = block.inner(area);
        f.render_widget(block, area);
        let inner = Rect::new(
            full.x,
            full.y,
            full.width,
            full.height
                .saturating_sub(u16::from(self.conflict.is_some())),
        );
        let shown = self.current_preview();
        let output_count=shown.map_or(0,|s|match &s.result{Preview::Ran(r)=>r.text.lines().count(),_=>1}+s.warnings.len());
        let conflict_count = self
            .conflict
            .as_ref()
            .map_or(0, |(_, v)| 1 + v.iter().filter(|(c, _)| *c != ' ').count());
        let total = self.approved.diff.len() + 2 + output_count + conflict_count;
        let offset = self
            .approved
            .review
            .offset
            .min(total.saturating_sub(inner.height as usize));
        let base = self.approved.diff.iter().cloned().map(|mut line| {
            line.style = design::role(
                p,
                if line.to_string().starts_with('+') {
                    Role::Ok
                } else {
                    Role::Error
                },
            );
            line
        });
        let note = [
            Line::raw(""),
            Line::raw(format!(
                "{} pending secret file(s); values masked",
                self.secrets.len()
            )),
        ];
        let output: Box<dyn Iterator<Item = Line<'static>> + '_> = match shown {
            Some(s) => {
                let result: Box<dyn Iterator<Item = Line<'static>> + '_> = match &s.result {
                    Preview::Ran(r) => Box::new(r.text.lines().map(|s| Line::raw(s.to_owned()))),
                    Preview::Skipped(why) => Box::new(std::iter::once(Line::raw(format!(
                        "Output not needed/unavailable: {why}"
                    )))),
                    Preview::Failed(why) => Box::new(std::iter::once(Line::styled(
                        why.clone(),
                        design::role(p, Role::Error),
                    ))),
                };
                Box::new(result.chain(s.warnings.iter().cloned().map(Line::raw)))
            }
            None => Box::new(std::iter::empty()),
        };
        let conflict: Box<dyn Iterator<Item = Line<'static>> + '_> = match &self.conflict {
            Some((_, diff)) => Box::new(
                std::iter::once(Line::styled(
                    "External file conflict",
                    design::role(p, Role::Error),
                ))
                .chain(
                    diff.iter()
                        .filter(|(c, _)| *c != ' ')
                        .map(|(c, s)| Line::raw(format!("{c} {s}"))),
                ),
            ),
            None => Box::new(std::iter::empty()),
        };
        // Collect visible review lines only. No quadratic line diff runs during draw.
        let lines = base
            .chain(note)
            .chain(output)
            .chain(conflict)
            .skip(offset)
            .take(inner.height as usize)
            .collect::<Vec<_>>();
        self.approved.review.offset = offset;
        f.render_widget(Paragraph::new(lines), inner);
        self.approved.review.draw(f, inner, total, p);
        if self.conflict.is_some() {
            let r = Rect::new(full.x, full.bottom().saturating_sub(1), full.width, 1);
            self.approved.buttons[9] = r;
            f.render_widget(
                Paragraph::new("[D Reload disk version]").style(design::accent(p)),
                r,
            );
        }
    }
    fn draw_router_list(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        let block = design::panel("Routers", p);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let [list, button] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner);
        let count = self
            .draft
            .as_ref()
            .map_or(0, |d| d.count(&key_path("routers")));
        self.approved.routers.offset = self
            .approved
            .routers
            .offset
            .min(count.saturating_sub(list.height as usize));
        for i in self.approved.routers.offset
            ..(self.approved.routers.offset + list.height as usize).min(count)
        {
            let name = self.router_name(i);
            let r = Rect::new(
                list.x,
                list.y + (i - self.approved.routers.offset) as u16,
                list.width,
                1,
            );
            self.approved.router_targets.push((r, i));
            f.render_widget(Paragraph::new(name).style(design::surface(p)), r);
        }
        self.approved.routers.draw(f, list, count, p);
        self.approved.buttons[3] = button;
        f.render_widget(
            Paragraph::new("[+ New router]").style(design::accent(p)),
            button,
        );
    }
    fn draw_router_form(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        let block = design::panel("Add router — not saved", p);
        let inner = block.inner(area);
        f.render_widget(block, area);
        let Some(form) = self.approved.form.as_ref() else {
            return;
        };
        let labelwidth = (inner.width / 4).min(20);
        let value = |row: u16| {
            Rect::new(
                inner.x + labelwidth,
                inner.y + row,
                inner.width.saturating_sub(labelwidth),
                1,
            )
        };
        for (row, index, label) in [
            (0, 0, "Name"),
            (1, 1, "Address"),
            (2, 2, "API port"),
            (3, 3, "Username"),
            (4, 4, "Password"),
            (5, 5, "Password file"),
        ] {
            f.render_widget(
                Paragraph::new(label),
                Rect::new(inner.x, inner.y + row, labelwidth, 1),
            );
            let r = value(row);
            self.approved.form_targets[index] = r;
            f.render_widget(
                Paragraph::new(format!(
                    "{}{}",
                    form.field(index),
                    if form.focus == index && form.editing {
                        "_"
                    } else {
                        ""
                    }
                ))
                .style(if form.focus == index {
                    design::selected(p)
                } else {
                    design::surface(p)
                }),
                r,
            );
        }
        let tls = value(2);
        let width = tls.width / 3;
        self.approved.form_targets[2] = Rect::new(tls.x, tls.y, width, 1);
        for (index, label) in [(6, "TLS"), (7, "Verify certificate")] {
            let r = Rect::new(tls.x + (index as u16 - 5) * width, tls.y, width, 1);
            self.approved.form_targets[index] = r;
            f.render_widget(
                Paragraph::new(format!(
                    "{label} [{}]",
                    if form.flags[index - 6] { "x" } else { " " }
                ))
                .style(design::accent(p)),
                r,
            );
        }
        f.render_widget(
            Paragraph::new("Source reads"),
            Rect::new(inner.x, inner.y + 6, labelwidth, 1),
        );
        let sources = value(6);
        let width = sources.width / 3;
        for (index, label) in [(8, "PPPoE"), (9, "DHCP"), (10, "Hotspot")] {
            let r = Rect::new(sources.x + (index as u16 - 8) * width, sources.y, width, 1);
            self.approved.form_targets[index] = r;
            f.render_widget(
                Paragraph::new(format!(
                    "[{}] {label}",
                    if form.flags[index - 6] { "x" } else { " " }
                ))
                .style(design::accent(p)),
                r,
            );
        }
        let r = Rect::new(inner.x, inner.y + 7, inner.width, 1);
        self.approved.form_targets[12] = r;
        f.render_widget(
            Paragraph::new("[Advanced options]").style(design::accent(p)),
            r,
        );
        if form.advanced {
            let r = value(8);
            self.approved.form_targets[11] = r;
            f.render_widget(
                Paragraph::new("TLS CA file"),
                Rect::new(inner.x, inner.y + 8, labelwidth, 1),
            );
            f.render_widget(
                Paragraph::new(form.field(11)).style(if form.focus == 11 {
                    design::selected(p)
                } else {
                    design::surface(p)
                }),
                r,
            );
        }
        f.render_widget(
            Paragraph::new("Other router defaults remain inherited."),
            Rect::new(inner.x, inner.y + 9, inner.width, 1),
        );
        for (index, label) in [(13, "[Add to draft]"), (14, "[Cancel form]")] {
            let r = Rect::new(
                inner.x + (index as u16 - 13) * inner.width / 2,
                inner.y + 11,
                inner.width / 2,
                1,
            );
            self.approved.form_targets[index] = r;
            f.render_widget(
                Paragraph::new(label).style(if form.focus == index {
                    design::selected(p)
                } else {
                    design::accent(p)
                }),
                r,
            );
        }
        f.render_widget(
            Paragraph::new("Draft only. No file or router change."),
            Rect::new(inner.x, inner.y + 12, inner.width, 1),
        );
    }
    fn draw_approved_popup(&mut self, f: &mut Frame, area: Rect, p: &Palette) {
        if self.modal.is_none()
            && matches!(
                self.input,
                Input::None
                    | Input::Field {
                        list_add: false,
                        ..
                    }
            )
        {
            return;
        }
        let mut lines = Vec::new();
        let mut choices = false;
        let title = if let Some(modal) = &self.modal {
            match modal {
                VModal::ConfirmApply => {
                    lines.extend([
                        Line::raw(format!(
                            "{} draft changes, {} secret files",
                            self.changed.len(),
                            self.secrets.len()
                        )),
                        Line::raw("Confirm backup, save, reload, and result verification?"),
                        Line::raw("No live change until confirmation"),
                    ]);
                    "Confirm apply"
                }
                VModal::RemoveRouter(r) => {
                    lines.push(Line::raw(format!(
                        "Remove {} from draft? Password file stays.",
                        self.router_name(*r)
                    )));
                    "Remove router"
                }
                VModal::RemoveServer(_, _) => {
                    lines.push(Line::raw("Remove this DHCP server from the draft?"));
                    "Remove server"
                }
            }
        } else {
            match &self.input {
                Input::Dropdown {
                    choices: items,
                    sel,
                    ..
                } => {
                    choices = true;
                    lines.extend(items.iter().enumerate().map(|(i, s)| {
                        Line::styled(
                            s.to_string(),
                            if i == *sel {
                                design::selected(p)
                            } else {
                                design::surface(p)
                            },
                        )
                    }));
                    "Choose value"
                }
                Input::List { path, sel, .. } => {
                    choices = true;
                    let values = self
                        .draft
                        .as_ref()
                        .and_then(|d| d.effective(path))
                        .and_then(|(v, _)| v.as_array().cloned())
                        .unwrap_or_default();
                    lines.extend(values.iter().enumerate().map(|(i, v)| {
                        Line::styled(
                            show(v),
                            if i == *sel {
                                design::selected(p)
                            } else {
                                design::surface(p)
                            },
                        )
                    }));
                    "Edit list"
                }
                Input::Secret { path, value } => {
                    lines.push(Line::raw(format!("File {}", path.display())));
                    lines.push(Line::raw("*".repeat(value.len()) + "_"));
                    "Secret — masked"
                }
                Input::Field { buf, .. } | Input::AddServer { buf, .. } => {
                    lines.push(Line::raw(format!("{buf}_")));
                    "Edit value"
                }
                Input::Client { step, fields, .. } => {
                    lines.push(Line::raw(format!("Field {}: {}_", step + 1, fields[*step])));
                    "Client override"
                }
                Input::AddRouter { .. } => {
                    lines.push(Line::raw(
                        "Legacy router wizard: use keyboard or cancel and choose New router",
                    ));
                    "Router input"
                }
                Input::None => "Confirm",
            }
        };
        let width = area.width.saturating_sub(4).min(100);
        let height = (lines.len() + 5)
            .min(area.height.saturating_sub(2) as usize)
            .max(4) as u16;
        let r = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        f.render_widget(Clear, r);
        let block = design::panel(title, p);
        let inner = block.inner(r);
        f.render_widget(block, r);
        let list = matches!(self.input, Input::List { .. });
        let [rows, ops, buttons] = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(u16::from(list)),
            Constraint::Length(1),
        ])
        .areas(inner);
        if list {
            let width = ops.width / 4;
            for (i, label) in ["[a Add]", "[d Delete]", "[K Up]", "[J Down]"]
                .into_iter()
                .enumerate()
            {
                let r = Rect::new(ops.x + i as u16 * width, ops.y, width, 1);
                self.approved.popup_ops[i] = r;
                f.render_widget(Paragraph::new(label).style(design::accent(p)), r);
            }
        }
        if choices {
            let selected = match &self.input {
                Input::Dropdown { sel, .. } | Input::List { sel, .. } => *sel,
                _ => 0,
            };
            if selected < self.approved.popup.offset {
                self.approved.popup.offset = selected;
            } else if rows.height > 0
                && selected >= self.approved.popup.offset + rows.height as usize
            {
                self.approved.popup.offset = selected + 1 - rows.height as usize;
            }
        }
        self.approved.popup.offset = self
            .approved
            .popup
            .offset
            .min(lines.len().saturating_sub(rows.height as usize));
        let total = lines.len();
        for (y, line) in lines
            .into_iter()
            .enumerate()
            .skip(self.approved.popup.offset)
            .take(rows.height as usize)
        {
            let r = Rect::new(
                rows.x,
                rows.y + (y - self.approved.popup.offset) as u16,
                rows.width,
                1,
            );
            if choices {
                self.approved.popup_rows.push((r, y));
            }
            f.render_widget(Paragraph::new(line), r);
        }
        self.approved.popup.draw(f, rows, total, p);
        self.approved.popup_yes = Rect::new(buttons.x, buttons.y, buttons.width / 2, 1);
        self.approved.popup_no = Rect::new(
            buttons.x + buttons.width / 2,
            buttons.y,
            buttons.width / 2,
            1,
        );
        f.render_widget(
            Paragraph::new(if self.modal.is_some() {
                "[y Confirm]"
            } else {
                "[Enter Save/choose]"
            })
            .style(design::accent(p)),
            self.approved.popup_yes,
        );
        f.render_widget(
            Paragraph::new("[Esc Cancel]").style(design::accent(p)),
            self.approved.popup_no,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }
    fn draw(v: &mut SettingsView) -> String {
        let mut t = Terminal::new(TestBackend::new(200, 40)).unwrap();
        v.clear_approved_areas();
        t.draw(|f| v.draw_approved(f, f.area(), &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
        t.backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect()
    }
    #[test]
    fn full_router_form_validates_before_draft_and_keeps_secrets_in_memory() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "").unwrap();
        let mut v = SettingsView::new(file.clone(), "", false);
        v.open_form();
        let secret = dir.path().join("new.password");
        let form = v.approved.form.as_mut().unwrap();
        form.fields[0] = "EDGE-05".into();
        form.fields[1] = "192.0.2.105".into();
        form.fields[2] = "invalid".into();
        form.fields[4] = secret.display().to_string();
        form.password = Secret::new("synthetic-password".into());
        let rendered = draw(&mut v);
        assert!(!rendered.contains("synthetic-password"));
        assert!(rendered.contains("****"));
        v.commit_form();
        assert_eq!(v.draft.as_ref().unwrap().text(), "");
        assert!(v.secrets.is_empty());
        assert!(!secret.exists());
        let form = v.approved.form.as_mut().unwrap();
        form.fields[2] = "8728".into();
        form.flags[0] = false;
        form.flags[2] = true;
        v.commit_form();
        assert!(v.approved.form.is_none());
        let cfg = validate::parse(&v.draft.as_ref().unwrap().text()).unwrap();
        assert_eq!(cfg.routers[0].port, 8728);
        assert!(!cfg.routers[0].tls);
        assert!(cfg.routers[0].pppoe.enabled);
        assert_eq!(v.secrets.len(), 1);
        assert!(!secret.exists());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "");
        let before = v.draft.as_ref().unwrap().text();
        v.open_form();
        let f = v.approved.form.as_mut().unwrap();
        f.fields[0] = "EDGE-05".into();
        f.fields[1] = "192.0.2.106".into();
        f.password = Secret::new("duplicate-password".into());
        v.commit_form();
        assert_eq!(v.draft.as_ref().unwrap().text(), before);
        assert_eq!(v.secrets.len(), 1);
        v.form_key(key(KeyCode::Esc));
        v.on_key(key(KeyCode::Char('u')));
        assert_eq!(v.draft.as_ref().unwrap().text(), "");
        assert!(
            v.secrets.is_empty(),
            "one undo removes the whole router and drops the pending secret"
        );
        v.open_form();
        v.approved.form.as_mut().unwrap().password = Secret::new("cancelled-password".into());
        v.form_key(key(KeyCode::Esc));
        assert!(v.approved.form.is_none());
        assert!(!secret.exists());
        v.read_only = true;
        v.open_form();
        assert!(v.approved.form.is_none());
    }
    #[test]
    fn mouse_apply_cannot_bypass_inline_edit_preview_or_modal_guards() {
        let mut v = SettingsView::new(
            PathBuf::new(),
            "[runtime]\nscan_interval_active=25\n",
            false,
        );
        v.select("runtime.scan_interval_active");
        v.draft
            .as_mut()
            .unwrap()
            .set(&key_path("runtime.scan_interval_active"), Value::from(30))
            .unwrap();
        v.changed_now();
        let SAction::Preview(job) = v.on_key(key(KeyCode::Char('v'))) else {
            panic!("preview job")
        };
        v.preview_done(PreviewDone {
            id: job.id,
            result: Preview::Skipped("runtime-only changes".into()),
            warnings: vec![],
        });
        draw(&mut v);
        let click = |r: Rect| MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        };
        v.approved_mouse(click(v.approved.buttons[1]));
        assert_eq!(v.modal, Some(VModal::ConfirmApply));
        let before = v.draft.as_ref().unwrap().text();
        draw(&mut v);
        v.approved_mouse(click(v.approved.key_targets[0].0));
        assert_eq!(v.draft.as_ref().unwrap().text(), before);
        assert!(v.modal.is_some());
        v.approved_mouse(click(v.approved.popup_no));
        assert!(v.modal.is_none());
        draw(&mut v);
        let i = v.groups[v.group]
            .keys
            .iter()
            .position(|(p, _)| path_text(p) == "runtime.scan_interval_active")
            .unwrap();
        let r = v
            .approved
            .key_targets
            .iter()
            .find(|(_, j)| *j == i)
            .unwrap()
            .0;
        v.approved_mouse(click(r));
        v.approved_key(key(KeyCode::Backspace));
        v.approved_key(key(KeyCode::Char('1')));
        assert!(
            v.current_preview().is_some(),
            "uncommitted field does not change the draft"
        );
        let action = v.approved_mouse(click(v.approved.buttons[1]));
        assert_eq!(action, SAction::None);
        assert!(v.current_preview().is_none());
        assert!(v.modal.is_none());
        assert_eq!(
            v.draft
                .as_ref()
                .unwrap()
                .effective(&key_path("runtime.scan_interval_active"))
                .unwrap()
                .0
                .as_integer(),
            Some(31)
        );
    }
}

#[cfg(test)]
mod exports {
    use super::*;
    #[test]
    #[ignore = "manual Settings fixture renderer; set LQOSYNC_RENDER_DIR"]
    fn export_settings_fixture_frames() {
        use crate::tui::app::{App, Tab};
        use ratatui::{Terminal, backend::TestBackend};
        let folder = PathBuf::from(std::env::var("LQOSYNC_RENDER_DIR").expect("set output folder"));
        std::fs::create_dir_all(&folder).unwrap();
        let text = "[runtime]\nmode='apply'\nscan_interval_active=25\nscan_interval_idle=120\n[[routers]]\nname='EDGE-01'\naddress='192.0.2.101'\nusername='fixture'\npassword_file='/fixture/EDGE-01.password'\n";
        let mut app = App::new(false, text.into(), None);
        app.tab = Tab::Settings;
        app.on_tick(1794190000);
        app.settings.open_form();
        let form = app.settings.approved.form.as_mut().unwrap();
        form.fields[0] = "EDGE-05".into();
        form.fields[1] = "192.0.2.105".into();
        form.fields[3] = "lqos-reader".into();
        form.fields[4] = "/etc/lqosync3/secrets/EDGE-05.password".into();
        form.password = Secret::new("synthetic-secret".into());
        form.flags[2] = true;
        form.flags[3] = true;
        let save = |app: &mut App, name: &str| {
            let mut t = Terminal::new(TestBackend::new(240, 40)).unwrap();
            t.draw(|f| app.draw(f)).unwrap();
            let cells=t.backend().buffer().content.iter().map(|c|serde_json::json!({"text":c.symbol(),"fg":format!("{:?}",c.fg),"bg":format!("{:?}",c.bg),"bold":c.modifier.contains(Modifier::BOLD)})).collect::<Vec<_>>();
            let json =
                serde_json::to_vec(&serde_json::json!({"width":240,"height":40,"cells":cells}))
                    .unwrap();
            assert!(!String::from_utf8_lossy(&json).contains("synthetic-secret"));
            std::fs::write(folder.join(format!("{name}.json")), json).unwrap();
        };
        save(&mut app, "settings-add-router-v1");
        app.settings.approved.form = None;
        app.settings.approved.router_list = false;
        app.settings.select("runtime.scan_interval_active");
        for (path, value) in [
            ("runtime.scan_interval_active", 30),
            ("runtime.scan_interval_idle", 180),
        ] {
            app.settings
                .draft
                .as_mut()
                .unwrap()
                .set(&key_path(path), Value::from(value))
                .unwrap();
        }
        app.settings.changed_now();
        let SAction::Preview(job) = app.settings.preview() else {
            panic!("fixture preview job")
        };
        app.settings.preview_done(PreviewDone {
            id: job.id,
            result: Preview::Skipped("No shaping-output change: runtime-only fixture".into()),
            warnings: vec![],
        });
        save(&mut app, "settings-values-review-v1");
    }
}

#[cfg(test)]
mod mouse_choice_tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    fn draw(v: &mut SettingsView) {
        v.clear_approved_areas();
        let mut t = Terminal::new(TestBackend::new(200, 40)).unwrap();
        t.draw(|f| v.draw_approved(f, f.area(), &Palette::new(crate::config::Theme::Dark)))
            .unwrap();
    }
    fn click(r: Rect) -> MouseEvent {
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: r.x,
            row: r.y,
            modifiers: KeyModifiers::NONE,
        }
    }
    #[test]
    fn dropdown_list_and_scroll_mouse_controls_use_existing_editor_actions() {
        let mut v = SettingsView::new(PathBuf::new(), "[runtime]\nmode='apply'\n", false);
        v.select("runtime.mode");
        v.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        draw(&mut v);
        let before = v.group;
        let background = v.approved.category_targets[2].0;
        v.approved_mouse(click(background));
        assert_eq!(v.group, before);
        assert!(matches!(v.input, Input::Dropdown { .. }));
        let row = v
            .approved
            .popup_rows
            .iter()
            .find(|(_, i)| *i == 0)
            .unwrap()
            .0;
        v.approved_mouse(click(row));
        assert_eq!(
            v.draft
                .as_ref()
                .unwrap()
                .effective(&key_path("runtime.mode"))
                .unwrap()
                .0
                .as_str(),
            Some("shadow")
        );
        v.select("speed.pppoe_sources");
        v.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        draw(&mut v);
        assert!(matches!(v.input, Input::List { .. }));
        let old = v
            .draft
            .as_ref()
            .unwrap()
            .effective(&key_path("speed.pppoe_sources"))
            .unwrap()
            .0
            .as_array()
            .unwrap()
            .len();
        v.approved_mouse(click(v.approved.popup_ops[0]));
        draw(&mut v);
        assert!(matches!(v.input, Input::Dropdown { .. }));
        let row = v.approved.popup_rows[0].0;
        v.approved_mouse(click(row));
        draw(&mut v);
        assert!(matches!(v.input, Input::List { .. }));
        assert_eq!(
            v.draft
                .as_ref()
                .unwrap()
                .effective(&key_path("speed.pppoe_sources"))
                .unwrap()
                .0
                .as_array()
                .unwrap()
                .len(),
            old + 1
        );
        v.approved_mouse(click(v.approved.popup_ops[1]));
        assert_eq!(
            v.draft
                .as_ref()
                .unwrap()
                .effective(&key_path("speed.pppoe_sources"))
                .unwrap()
                .0
                .as_array()
                .unwrap()
                .len(),
            old
        );
    }
}
