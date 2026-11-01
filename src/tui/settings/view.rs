//! The Settings screen: groups, keys, help; the inputs; preview, confirm, apply, verify.

use super::apply::{ApplyError, Verify, apply, verify};
use super::draft::{
    Draft, KeyPath, Origin, Seg, line_diff, parse_value, path_text, valid_router_name,
};
use super::meta::{self, KeyMeta, Kind};
use super::preview::{self, Preview};
use super::secrets::{Pending, Secret};
use super::validate;
use crate::tui::dashboard::Sent;
use ratatui::Frame;
use ratatui::crossterm::event::KeyModifiers;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use serde_json::Value as Json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use toml_edit::Value;

/// What the main loop must do for the Settings tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SAction {
    None,
    /// Run this preview in a worker thread, then call `preview_done`.
    Preview(PreviewJob),
    /// The operator confirmed the apply: call `do_apply`, send SIGHUP, then `reload_sent`.
    Apply,
}

/// The input of the preview worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewJob {
    pub id: u64,
    /// The file text at the start of the edit, and the new text.
    pub old_text: String,
    pub new_text: String,
    /// The secret files that the apply writes.
    pub typed: HashSet<PathBuf>,
    pub changed: Vec<String>,
    /// `config_digest` of `status.json`: the loaded config.
    pub loaded_digest: Option<String>,
}

#[derive(Debug)]
pub struct PreviewDone {
    pub id: u64,
    pub result: Preview,
    pub warnings: Vec<String>,
}

/// The preview worker: validate (with IO), then the preview. Runs outside the draw loop.
pub fn run_preview(job: PreviewJob) -> PreviewDone {
    let done = |result, warnings| PreviewDone {
        id: job.id,
        result,
        warnings,
    };
    let (new, mut warnings) = match validate::validate(&job.new_text, &job.typed) {
        Ok(x) => x,
        Err(e) => return done(Preview::Failed(format!("validate: {e}")), vec![]),
    };
    let old = match validate::parse(&job.old_text) {
        Ok(c) => c,
        Err(e) => {
            // The operator can fix a file that fails the config rules: no output preview then.
            warnings.push(format!("the current file fails the config rules: {e}"));
            return done(
                Preview::Skipped("the current file is not valid, so no output preview".into()),
                warnings,
            );
        }
    };
    if job
        .loaded_digest
        .as_deref()
        .is_some_and(|d| d != crate::cli::config_digest(job.old_text.as_bytes()))
    {
        warnings.push(
            "the file on disk is not the loaded config: the preview starts from the file".into(),
        );
    }
    if !preview::output_keys_changed(&job.changed) {
        return done(
            Preview::Skipped("no key that changes the LibreQoS files changed".into()),
            warnings,
        );
    }
    let snapshot =
        std::fs::read_to_string(Path::new(&old.runtime.state_dir).join("snapshot.json")).ok();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let result = preview::preview(
        &old,
        &new,
        snapshot.as_deref(),
        now,
        old.tui.preview_max_age,
    );
    done(result, warnings)
}

#[derive(Debug)]
enum Input {
    None,
    Field {
        path: KeyPath,
        kind: Kind,
        buf: String,
        list_add: bool,
    },
    Dropdown {
        path: KeyPath,
        choices: &'static [&'static str],
        sel: usize,
        list_add: bool,
    },
    List {
        path: KeyPath,
        inner: Kind,
        sel: usize,
    },
    Secret {
        path: PathBuf,
        value: Secret,
    },
    AddRouter {
        step: usize,
        fields: [String; 3],
        buf: String,
        pw: Secret,
    },
    AddServer {
        router: usize,
        buf: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VModal {
    ConfirmApply,
    RemoveRouter(usize),
    RemoveServer(usize, usize),
}

#[derive(Debug)]
struct Shown {
    text: String,
    rev: u64,
    result: Preview,
    warnings: Vec<String>,
}

#[derive(Debug)]
enum Stage {
    Edit,
    Verifying {
        sent: Sent,
        want: String,
        loaded_seq: Option<u64>,
    },
    Result(String),
}

struct Group {
    title: String,
    keys: Vec<(KeyPath, &'static KeyMeta)>,
    router: Option<usize>,
    server: Option<(usize, usize)>,
}

pub struct SettingsView {
    path: PathBuf,
    pub draft: Option<Draft>,
    load_error: Option<String>,
    raw: String,
    pub read_only: bool,
    groups: Vec<Group>,
    group: usize,
    key: usize,
    /// 0 groups, 1 keys.
    focus: u8,
    input: Input,
    pub modal: Option<VModal>,
    pub secrets: Vec<Pending>,
    secrets_rev: u64,
    pub message: Option<String>,
    changed: Vec<String>,
    preview_id: u64,
    running: Option<(u64, String, u64)>,
    shown: Option<Shown>,
    stage: Stage,
    /// The `config_digest` of `status.json`.
    pub loaded_digest: Option<String>,
    /// The panels of the last draw: groups, keys, help.
    pub areas: [Rect; 3],
    /// The scroll of the right panel (`[` and `]`).
    pub scroll: u16,
    /// The file on disk changed: its text and the diff of the start text and the disk text.
    conflict: Option<(String, Vec<(char, String)>)>,
    /// The backup of the last apply.
    last_backup: Option<PathBuf>,
}

fn show(v: &Value) -> String {
    v.clone().decorated("", "").to_string()
}

/// The meta path with each `*` replaced by the next index.
fn concrete(meta_path: &str, idx: &[usize]) -> KeyPath {
    let mut it = idx.iter();
    meta_path
        .split('.')
        .map(|s| match s {
            "*" => Seg::Index(*it.next().unwrap_or(&0)),
            k => Seg::Key(k.to_string()),
        })
        .collect()
}

fn key_path(text: &str) -> KeyPath {
    text.split('.')
        .map(|x| {
            x.parse::<usize>()
                .map_or(Seg::Key(x.to_string()), Seg::Index)
        })
        .collect()
}

impl SettingsView {
    pub fn new(path: PathBuf, text: &str, read_only: bool) -> Self {
        let (draft, load_error) = match Draft::new(text) {
            Ok(d) => (Some(d), None),
            Err(e) => (None, Some(format!("{e:#}"))),
        };
        let mut v = SettingsView {
            path,
            draft,
            load_error,
            raw: text.to_string(),
            read_only,
            groups: Vec::new(),
            group: 0,
            key: 0,
            focus: 0,
            input: Input::None,
            modal: None,
            secrets: Vec::new(),
            secrets_rev: 0,
            message: None,
            changed: Vec::new(),
            preview_id: 0,
            running: None,
            shown: None,
            stage: Stage::Edit,
            loaded_digest: None,
            areas: [Rect::default(); 3],
            scroll: 0,
            conflict: None,
            last_backup: None,
        };
        v.after_change();
        v
    }

    /// Changes or typed secrets that are not applied.
    pub fn dirty(&self) -> bool {
        !self.changed.is_empty() || !self.secrets.is_empty()
    }

    /// An input or a modal is open: it takes all keys.
    pub fn capturing(&self) -> bool {
        !matches!(self.input, Input::None) || self.modal.is_some()
    }

    fn after_change(&mut self) {
        self.changed = self.draft.as_ref().map(Draft::changed).unwrap_or_default();
        self.rebuild_groups();
        if let Stage::Result(_) = self.stage {
            self.stage = Stage::Edit;
        }
        // A typed secret is only for a file that a key of the draft names.
        if let Some(d) = &self.draft {
            let mut used: HashSet<PathBuf> = (0..d.count(&[Seg::Key("routers".into())]))
                .filter_map(|i| d.effective(&concrete("routers.*.password_file", &[i])))
                .filter_map(|(v, _)| v.as_str().map(PathBuf::from))
                .collect();
            if let Some((v, _)) = d.effective(&key_path("telegram.bot_token_file"))
                && let Some(t) = v.as_str()
            {
                used.insert(PathBuf::from(t));
            }
            let dropped: Vec<String> = self
                .secrets
                .iter()
                .filter(|p| !used.contains(&p.path))
                .map(|p| p.path.display().to_string())
                .collect();
            if !dropped.is_empty() {
                self.secrets.retain(|p| used.contains(&p.path));
                self.secrets_rev += 1;
                self.message = Some(format!(
                    "a typed secret was dropped: no key names {} now",
                    dropped.join(", ")
                ));
            }
        }
    }

    fn rebuild_groups(&mut self) {
        let mut groups: Vec<Group> = Vec::new();
        for m in meta::all().iter().filter(|m| !m.path.contains('*')) {
            let title = m.path.rsplit_once('.').map_or("", |(t, _)| t).to_string();
            let p = concrete(&m.path, &[]);
            match groups.iter_mut().find(|g| g.title == title) {
                Some(g) => g.keys.push((p, m)),
                None => groups.push(Group {
                    title,
                    keys: vec![(p, m)],
                    router: None,
                    server: None,
                }),
            }
        }
        if let Some(d) = &self.draft {
            let routers = vec![Seg::Key("routers".into())];
            for i in 0..d.count(&routers) {
                let name = d
                    .get(&concrete("routers.*.name", &[i]))
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| format!("#{i}"));
                let pick = |prefix: &str| -> Vec<(KeyPath, &'static KeyMeta)> {
                    meta::all()
                        .iter()
                        .filter_map(|m| {
                            let rest = m.path.strip_prefix(prefix)?;
                            (!rest.contains('.')).then(|| (concrete(&m.path, &[i]), m))
                        })
                        .collect()
                };
                let mut top = pick("routers.*.");
                let first = ["name", "address", "username", "password_file"];
                top.sort_by_key(|(_, m)| {
                    let k = m.path.rsplit('.').next().unwrap_or_default();
                    first.iter().position(|f| *f == k).unwrap_or(first.len())
                });
                groups.push(Group {
                    title: format!("router {name}"),
                    keys: top,
                    router: Some(i),
                    server: None,
                });
                for t in ["pppoe", "hotspot", "dhcp", "exclude"] {
                    groups.push(Group {
                        title: format!("router {name} {t}"),
                        keys: pick(&format!("routers.*.{t}.")),
                        router: Some(i),
                        server: None,
                    });
                }
                let servers = concrete("routers.*.dhcp.servers", &[i]);
                for j in 0..d.count(&servers) {
                    let sname = d
                        .get(&concrete("routers.*.dhcp.servers.*.name", &[i, j]))
                        .and_then(|v| v.as_str().map(str::to_string))
                        .unwrap_or_else(|| format!("#{j}"));
                    let keys = meta::all()
                        .iter()
                        .filter(|m| m.path.starts_with("routers.*.dhcp.servers.*."))
                        .map(|m| (concrete(&m.path, &[i, j]), m))
                        .collect();
                    groups.push(Group {
                        title: format!("router {name} dhcp server {sname}"),
                        keys,
                        router: Some(i),
                        server: Some((i, j)),
                    });
                }
            }
        }
        self.groups = groups;
        self.group = self.group.min(self.groups.len().saturating_sub(1));
        let n = self.groups.get(self.group).map_or(0, |g| g.keys.len());
        self.key = self.key.min(n.saturating_sub(1));
    }

    /// Select the key of a path (`runtime.mode`, `routers.0.name`). `false` if it is not shown.
    pub fn select(&mut self, path: &str) -> bool {
        let want = key_path(path);
        for (gi, g) in self.groups.iter().enumerate() {
            if let Some(ki) = g.keys.iter().position(|(p, _)| *p == want) {
                (self.group, self.key, self.focus) = (gi, ki, 1);
                return true;
            }
        }
        false
    }

    fn current(&self) -> Option<&(KeyPath, &'static KeyMeta)> {
        self.groups.get(self.group)?.keys.get(self.key)
    }

    /// The value in the file, else the default.
    fn effective(&self, p: &[Seg], m: &KeyMeta) -> Option<Value> {
        match &self.draft {
            Some(d) => d.effective(p).map(|(v, _)| v),
            None => m.default.as_deref().and_then(parse_value),
        }
    }

    fn blocked(&mut self) -> bool {
        if self.read_only {
            self.message = Some("read-only (not root): start the TUI with sudo to edit".into());
        } else if self.draft.is_none() {
            self.message = Some("the config file has a TOML error: fix the file by hand".into());
        }
        self.read_only || self.draft.is_none()
    }

    /// After a draft change: the config rules (no IO). An error names the key; the value stays.
    fn changed_now(&mut self) {
        self.after_change();
        if let Some(d) = &self.draft
            && let Err(e) = validate::parse(&d.text())
        {
            self.message = Some(e);
        }
    }

    fn result<T>(&mut self, r: anyhow::Result<T>) -> Option<T> {
        match r {
            Ok(x) => Some(x),
            Err(e) => {
                self.message = Some(format!("{e:#}"));
                None
            }
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> SAction {
        if let Some(m) = self.modal.take() {
            return self.modal_key(m, k);
        }
        if !matches!(self.input, Input::None) {
            self.input_key(k);
            return SAction::None;
        }
        self.message = None;
        match k.code {
            KeyCode::Left => self.focus = 0,
            KeyCode::Right => self.focus = 1,
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End => self.move_sel(k.code),
            KeyCode::Enter | KeyCode::Char(' ') if self.focus == 1 => self.edit(),
            KeyCode::Enter => self.focus = 1,
            KeyCode::Char('r') => self.reset(),
            KeyCode::Char('u') => {
                if !self.blocked() {
                    let ok = self.draft.as_mut().is_some_and(Draft::undo);
                    if !ok {
                        self.message = Some("no change to undo".into());
                    }
                    self.after_change();
                }
            }
            KeyCode::Char('P') => self.ask_secret(),
            KeyCode::Char('+') => self.add(),
            KeyCode::Char('-') => self.remove(),
            KeyCode::Char('v') => return self.preview(),
            KeyCode::Char('A') => self.ask_apply(),
            KeyCode::Char(']') => self.scroll = self.scroll.saturating_add(10),
            KeyCode::Char('[') => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Char('D') => self.restart_from_disk(),
            _ => {}
        }
        SAction::None
    }

    /// After a "file changed on disk": drop the edits and start from the disk text.
    fn restart_from_disk(&mut self) {
        let Some((disk, _)) = self.conflict.take() else {
            self.message = Some("D starts from the file on disk after a changed-file error".into());
            return;
        };
        match Draft::new(&disk) {
            Ok(d) => {
                self.draft = Some(d);
                self.raw = disk;
                self.shown = None;
                self.after_change();
                self.message = Some("the draft starts again from the file on disk".into());
            }
            Err(e) => self.message = Some(format!("the file on disk: {e:#}")),
        }
    }

    fn move_sel(&mut self, code: KeyCode) {
        let (sel, len) = if self.focus == 0 {
            (&mut self.group, self.groups.len())
        } else {
            let n = self.groups.get(self.group).map_or(0, |g| g.keys.len());
            (&mut self.key, n)
        };
        let last = len.saturating_sub(1);
        *sel = match code {
            KeyCode::Up => sel.saturating_sub(1),
            KeyCode::Down => (*sel + 1).min(last),
            KeyCode::PageUp => sel.saturating_sub(10),
            KeyCode::PageDown => (*sel + 10).min(last),
            KeyCode::Home => 0,
            _ => last,
        };
        if self.focus == 0 {
            self.key = 0;
        }
    }

    fn edit(&mut self) {
        let Some((p, m)) = self.current().cloned() else {
            return;
        };
        if m.read_only {
            self.message = Some(format!("{}: {}", m.path, m.help.replace('\n', " ")));
            return;
        }
        if self.blocked() {
            return;
        }
        let cur = self.effective(&p, m);
        match &m.kind {
            Kind::Bool => {
                let on = cur.and_then(|v| v.as_bool()).unwrap_or(false);
                let r = self.draft.as_mut().map(|d| d.set(&p, Value::from(!on)));
                if let Some(r) = r {
                    self.result(r);
                }
                self.changed_now();
            }
            Kind::Choice(c) => {
                let now = cur.as_ref().and_then(|v| v.as_str()).unwrap_or_default();
                let sel = c.iter().position(|x| *x == now).unwrap_or(0);
                self.input = Input::Dropdown {
                    path: p,
                    choices: c,
                    sel,
                    list_add: false,
                };
            }
            Kind::List(inner) => {
                self.input = Input::List {
                    path: p,
                    inner: (**inner).clone(),
                    sel: 0,
                }
            }
            kind => {
                let buf = match &cur {
                    Some(Value::String(s)) => s.value().clone(),
                    Some(v) => show(v),
                    None => String::new(),
                };
                self.input = Input::Field {
                    path: p,
                    kind: kind.clone(),
                    buf,
                    list_add: false,
                };
            }
        }
    }

    fn reset(&mut self) {
        let Some((p, m)) = self.current().cloned() else {
            return;
        };
        if m.read_only || self.blocked() {
            if m.read_only {
                self.message = Some(format!("{}: read-only here", m.path));
            }
            return;
        }
        if !m.optional && m.default.is_none() {
            self.message = Some(format!("{}: a required key has no default", path_text(&p)));
            return;
        }
        let r = self.draft.as_mut().map(|d| d.reset(&p));
        if let Some(r) = r {
            self.result(r);
        }
        self.changed_now();
    }

    fn ask_secret(&mut self) {
        if self.blocked() {
            return;
        }
        let Some((p, _)) = self.current().cloned() else {
            return;
        };
        let target = match p.first() {
            Some(Seg::Key(k)) if k == "routers" => p.get(1).map(|i| {
                vec![
                    Seg::Key("routers".into()),
                    i.clone(),
                    Seg::Key("password_file".into()),
                ]
            }),
            Some(Seg::Key(k)) if k == "telegram" => Some(key_path("telegram.bot_token_file")),
            _ => None,
        };
        let Some(t) = target else {
            self.message = Some(
                "P sets a router password (in a router group) or the Telegram token (in telegram)"
                    .into(),
            );
            return;
        };
        let file = meta::find(&path_text(&t))
            .and_then(|m| self.effective(&t, m))
            .and_then(|v| v.as_str().map(str::to_string));
        match file {
            Some(f) if !f.is_empty() => {
                self.input = Input::Secret {
                    path: PathBuf::from(f),
                    value: Secret::new(String::new()),
                }
            }
            _ => self.message = Some(format!("{}: set the file name first", path_text(&t))),
        }
    }

    fn add_pending(&mut self, path: PathBuf, value: Secret) {
        self.secrets.retain(|p| p.path != path);
        self.secrets.push(Pending { path, value });
        self.secrets_rev += 1;
    }

    fn add(&mut self) {
        if self.blocked() {
            return;
        }
        let g = self.groups.get(self.group);
        let dhcp = g.is_some_and(|g| g.server.is_some() || g.title.ends_with(" dhcp"));
        match g.and_then(|g| g.router) {
            Some(r) if dhcp => {
                self.input = Input::AddServer {
                    router: r,
                    buf: String::new(),
                }
            }
            _ => {
                self.input = Input::AddRouter {
                    step: 0,
                    fields: Default::default(),
                    buf: String::new(),
                    pw: Secret::new(String::new()),
                }
            }
        }
    }

    fn remove(&mut self) {
        if self.blocked() {
            return;
        }
        let Some(g) = self.groups.get(self.group) else {
            return;
        };
        self.modal = match (g.server, g.router) {
            (Some((r, s)), _) => Some(VModal::RemoveServer(r, s)),
            (None, Some(r)) => Some(VModal::RemoveRouter(r)),
            _ => {
                self.message = Some("- removes a router or a DHCP server: select its group".into());
                None
            }
        };
    }

    fn modal_key(&mut self, m: VModal, k: KeyEvent) -> SAction {
        if k.code != KeyCode::Char('y') {
            self.message = Some("cancelled".into());
            return SAction::None;
        }
        match m {
            VModal::ConfirmApply => return SAction::Apply,
            VModal::RemoveRouter(r) => {
                let pf = self.router_password_file(r);
                let res = self.draft.as_mut().map(|d| d.remove_router(r));
                if let Some(res) = res
                    && self.result(res).is_some()
                {
                    self.message = Some(format!(
                        "router removed; the file {pf} stays: delete it by hand if you do not need it"
                    ));
                }
            }
            VModal::RemoveServer(r, s) => {
                let res = self.draft.as_mut().map(|d| d.remove_server(r, s));
                if let Some(res) = res {
                    self.result(res);
                }
            }
        }
        self.group = self.group.saturating_sub(1);
        self.changed_now();
        SAction::None
    }

    fn router_password_file(&self, r: usize) -> String {
        self.draft
            .as_ref()
            .and_then(|d| d.get(&concrete("routers.*.password_file", &[r])))
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| "?".into())
    }

    fn input_key(&mut self, k: KeyEvent) {
        // Ctrl+C cancels an input; another Ctrl or Alt key types nothing.
        if k.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && let KeyCode::Char(c) = k.code
        {
            if c == 'c' && k.modifiers.contains(KeyModifiers::CONTROL) {
                self.input = Input::None;
                self.message = Some("cancelled".into());
            }
            return;
        }
        let input = std::mem::replace(&mut self.input, Input::None);
        self.input = match input {
            Input::None => Input::None,
            Input::Field {
                path,
                kind,
                mut buf,
                list_add,
            } => match k.code {
                KeyCode::Esc => Input::None,
                KeyCode::Enter => {
                    self.commit_field(&path, &kind, &buf, list_add);
                    Input::None
                }
                KeyCode::Backspace => {
                    buf.pop();
                    Input::Field {
                        path,
                        kind,
                        buf,
                        list_add,
                    }
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    Input::Field {
                        path,
                        kind,
                        buf,
                        list_add,
                    }
                }
                _ => Input::Field {
                    path,
                    kind,
                    buf,
                    list_add,
                },
            },
            Input::Dropdown {
                path,
                choices,
                mut sel,
                list_add,
            } => match k.code {
                KeyCode::Esc => Input::None,
                KeyCode::Up => {
                    sel = sel.saturating_sub(1);
                    Input::Dropdown {
                        path,
                        choices,
                        sel,
                        list_add,
                    }
                }
                KeyCode::Down => {
                    sel = (sel + 1).min(choices.len().saturating_sub(1));
                    Input::Dropdown {
                        path,
                        choices,
                        sel,
                        list_add,
                    }
                }
                KeyCode::Enter => {
                    let v = Value::from(choices[sel]);
                    let r = self.draft.as_mut().map(|d| {
                        if list_add {
                            d.list_push(&path, v)
                        } else {
                            d.set(&path, v)
                        }
                    });
                    if let Some(r) = r {
                        self.result(r);
                    }
                    self.changed_now();
                    if list_add {
                        Input::List {
                            path,
                            inner: Kind::Choice(choices),
                            sel: 0,
                        }
                    } else {
                        Input::None
                    }
                }
                _ => Input::Dropdown {
                    path,
                    choices,
                    sel,
                    list_add,
                },
            },
            Input::List {
                path,
                inner,
                mut sel,
            } => {
                let len = self
                    .draft
                    .as_ref()
                    .and_then(|d| d.effective(&path))
                    .and_then(|(v, _)| v.as_array().map(|a| a.len()))
                    .unwrap_or(0);
                let last = len.saturating_sub(1);
                let list = |sel| Input::List {
                    path: path.clone(),
                    inner: inner.clone(),
                    sel,
                };
                match k.code {
                    KeyCode::Esc | KeyCode::Enter => Input::None,
                    KeyCode::Up => list(sel.saturating_sub(1)),
                    KeyCode::Down => list((sel + 1).min(last)),
                    KeyCode::Char('a') => match &inner {
                        Kind::Choice(c) => Input::Dropdown {
                            path: path.clone(),
                            choices: c,
                            sel: 0,
                            list_add: true,
                        },
                        k => Input::Field {
                            path: path.clone(),
                            kind: k.clone(),
                            buf: String::new(),
                            list_add: true,
                        },
                    },
                    KeyCode::Char('d') | KeyCode::Char('K') | KeyCode::Char('J') if len > 0 => {
                        let r = self.draft.as_mut().map(|d| match k.code {
                            KeyCode::Char('d') => d.list_remove(&path, sel),
                            KeyCode::Char('K') => d.list_move(&path, sel, sel.saturating_sub(1)),
                            _ => d.list_move(&path, sel, (sel + 1).min(last)),
                        });
                        if let Some(r) = r {
                            self.result(r);
                        }
                        sel = match k.code {
                            KeyCode::Char('K') => sel.saturating_sub(1),
                            KeyCode::Char('J') => (sel + 1).min(last),
                            _ => sel.min(last.saturating_sub(1)),
                        };
                        self.changed_now();
                        list(sel)
                    }
                    _ => list(sel),
                }
            }
            Input::Secret { path, mut value } => match k.code {
                KeyCode::Esc => Input::None,
                KeyCode::Enter if value.is_empty() => {
                    self.message = Some("an empty secret is not allowed".into());
                    Input::Secret { path, value }
                }
                KeyCode::Enter => {
                    self.message = Some(format!(
                        "a new secret for {} is ready; it is written at apply",
                        path.display()
                    ));
                    self.add_pending(path, value);
                    Input::None
                }
                KeyCode::Backspace => {
                    value.pop();
                    Input::Secret { path, value }
                }
                KeyCode::Char(c) => {
                    value.push(c);
                    Input::Secret { path, value }
                }
                _ => Input::Secret { path, value },
            },
            Input::AddServer { router, mut buf } => match k.code {
                KeyCode::Esc => Input::None,
                KeyCode::Enter => {
                    let r = self
                        .draft
                        .as_mut()
                        .map(|d| d.add_server(router, buf.trim()));
                    if let Some(r) = r {
                        self.result(r);
                    }
                    self.changed_now();
                    Input::None
                }
                KeyCode::Backspace => {
                    buf.pop();
                    Input::AddServer { router, buf }
                }
                KeyCode::Char(c) => {
                    buf.push(c);
                    Input::AddServer { router, buf }
                }
                _ => Input::AddServer { router, buf },
            },
            Input::AddRouter {
                step,
                mut fields,
                mut buf,
                mut pw,
            } => match (k.code, step) {
                (KeyCode::Esc, _) => Input::None,
                (KeyCode::Enter, 0) if !valid_router_name(buf.trim()) => {
                    self.message = Some("router name: use 1 to 64 of A-Z a-z 0-9 . _ -".into());
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                (KeyCode::Enter, 0..=2) => {
                    fields[step] = buf.trim().to_string();
                    Input::AddRouter {
                        step: step + 1,
                        fields,
                        buf: String::new(),
                        pw,
                    }
                }
                (KeyCode::Enter, _) if pw.is_empty() => {
                    self.message = Some("the router needs a password".into());
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                (KeyCode::Enter, _) => {
                    let file = format!("/etc/lqosync3/secrets/{}.password", fields[0]);
                    let r = self
                        .draft
                        .as_mut()
                        .map(|d| d.add_router(&fields[0], &fields[1], &fields[2], &file));
                    if let Some(r) = r
                        && self.result(r).is_some()
                    {
                        self.add_pending(PathBuf::from(file), pw);
                    }
                    self.changed_now();
                    Input::None
                }
                (KeyCode::Backspace, 3) => {
                    pw.pop();
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                (KeyCode::Char(c), 3) => {
                    pw.push(c);
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                (KeyCode::Backspace, _) => {
                    buf.pop();
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                (KeyCode::Char(c), _) => {
                    buf.push(c);
                    Input::AddRouter {
                        step,
                        fields,
                        buf,
                        pw,
                    }
                }
                _ => Input::AddRouter {
                    step,
                    fields,
                    buf,
                    pw,
                },
            },
        };
    }

    fn commit_field(&mut self, path: &[Seg], kind: &Kind, buf: &str, list_add: bool) {
        let at = path_text(path);
        let v = match kind {
            Kind::Int => match buf.trim().parse::<i64>() {
                Ok(n) => Value::from(n),
                Err(_) => {
                    self.message = Some(format!("{at}: \"{buf}\" is not a whole number"));
                    return;
                }
            },
            Kind::Float => match buf.trim().parse::<f64>() {
                Ok(n) if n.is_finite() => Value::from(n),
                _ => {
                    self.message = Some(format!("{at}: \"{buf}\" is not a number"));
                    return;
                }
            },
            _ => Value::from(buf),
        };
        let r = self.draft.as_mut().map(|d| {
            if list_add {
                d.list_push(path, v)
            } else {
                d.set(path, v)
            }
        });
        if let Some(r) = r {
            self.result(r);
        }
        self.changed_now();
        if list_add {
            self.input = Input::List {
                path: path.to_vec(),
                inner: kind.clone(),
                sel: 0,
            };
        }
    }

    fn preview(&mut self) -> SAction {
        if self.blocked() {
            return SAction::None;
        }
        if !self.dirty() {
            self.message = Some("no change to preview".into());
            return SAction::None;
        }
        let d = self.draft.as_ref().expect("blocked() checks the draft");
        self.preview_id += 1;
        let text = d.text();
        self.running = Some((self.preview_id, text.clone(), self.secrets_rev));
        self.message = Some("preview running".into());
        SAction::Preview(PreviewJob {
            id: self.preview_id,
            old_text: d.original().to_string(),
            new_text: text,
            typed: self.secrets.iter().map(|p| p.path.clone()).collect(),
            changed: self.changed.clone(),
            loaded_digest: self.loaded_digest.clone(),
        })
    }

    pub fn preview_done(&mut self, d: PreviewDone) {
        match self.running.take() {
            Some((id, text, rev)) if id == d.id => {
                self.message = None;
                self.scroll = 0;
                self.shown = Some(Shown {
                    text,
                    rev,
                    result: d.result,
                    warnings: d.warnings,
                });
            }
            other => self.running = other,
        }
    }

    /// The preview of the current draft and secrets, if there is one.
    fn current_preview(&self) -> Option<&Shown> {
        let d = self.draft.as_ref()?;
        self.shown
            .as_ref()
            .filter(|s| s.rev == self.secrets_rev && s.text == d.text())
    }

    fn ask_apply(&mut self) {
        if self.blocked() {
            return;
        }
        if !self.dirty() {
            self.message = Some("no change to apply".into());
            return;
        }
        match self.current_preview().map(|s| &s.result) {
            None => self.message = Some("apply needs a preview of these changes: press v".into()),
            Some(Preview::Failed(_)) => {
                self.message = Some("the preview failed: fix the cause, then press v again".into())
            }
            Some(_) => self.modal = Some(VModal::ConfirmApply),
        }
    }

    /// Write the secrets and the file (called by the main loop after `SAction::Apply`).
    /// On success the draft starts again from the new text; returns the new digest.
    pub fn do_apply(&mut self, state_dir: &Path, keep: usize, now: u64) -> Result<String, String> {
        let d = self.draft.as_ref().ok_or("no draft")?;
        let text = d.text();
        match apply(
            &self.path,
            &d.start_digest,
            &text,
            &self.secrets,
            state_dir,
            keep,
            now,
        ) {
            Ok(digest) => {
                self.last_backup = Some(
                    state_dir
                        .join("config-backups")
                        .join(format!("{now:012}.toml")),
                );
                self.conflict = None;
                self.draft = Draft::new(&text).ok();
                self.raw = text;
                self.secrets.clear();
                self.secrets_rev += 1;
                self.shown = None;
                self.after_change();
                Ok(digest)
            }
            Err(ApplyError::Changed { disk }) => {
                let diff = line_diff(d.original(), &disk);
                self.conflict = Some((disk, diff));
                self.scroll = 0;
                let m = "not applied: another person changed the file on disk; nothing was written. The right panel shows the change on disk: D starts again from it".to_string();
                self.message = Some(m.clone());
                Err(m)
            }
            Err(ApplyError::Io(e)) => {
                let m = format!("not applied: {e}");
                self.message = Some(m.clone());
                Err(m)
            }
        }
    }

    /// The SIGHUP after a written file.
    pub fn reload_sent(&mut self, sent: Result<Sent, String>, want: String) {
        match sent {
            Ok(sent) => {
                self.message = Some("file written; SIGHUP sent; waiting for status.json".into());
                self.stage = Stage::Verifying {
                    sent,
                    want,
                    loaded_seq: None,
                };
            }
            Err(e) => {
                self.stage = Stage::Result(format!(
                    "not verified: the file is written, but SIGHUP was not sent ({e}); run: sudo systemctl reload lqosync3"
                ));
            }
        }
    }

    /// New `status.json` data.
    pub fn on_status(&mut self, status: Option<&Json>, now: u64, timeout: u64) {
        self.loaded_digest = status.and_then(|s| s["config_digest"].as_str().map(str::to_string));
        let backup = self.backup_text();
        let Stage::Verifying {
            sent,
            want,
            loaded_seq,
        } = &mut self.stage
        else {
            return;
        };
        let next = match verify(sent, want, *loaded_seq, status, now, timeout) {
            Verify::WaitReload => None,
            Verify::Loaded => {
                if loaded_seq.is_none() {
                    *loaded_seq = status.and_then(|s| s["cycle_seq"].as_u64()).or(Some(0));
                }
                self.message =
                    Some("applied: the service loaded the new config; the first cycle runs".into());
                None
            }
            Verify::Done(m) => Some(m),
            Verify::Rejected(e) => Some(format!(
                "not applied: the service rejected the file and keeps the old config in memory: {e}. The new file is on disk and loads at the next restart: fix it and apply again, or restore the backup {}",
                backup
            )),
            Verify::NotVerified { want, have, why } => Some(format!(
                "not verified: {why} (want {want}, status.json has {have}). The new file is on disk; backup {}",
                backup
            )),
        };
        if let Some(m) = next {
            self.stage = Stage::Result(m);
        }
    }

    fn backup_text(&self) -> String {
        self.last_backup
            .as_ref()
            .map_or("unknown".into(), |p| p.display().to_string())
    }

    /// The config of the draft text: the apply takes `state_dir` and `keep_backups` from it.
    pub fn apply_target(&self) -> Option<(PathBuf, usize)> {
        let c = validate::parse(&self.draft.as_ref()?.text()).ok()?;
        Some((PathBuf::from(c.runtime.state_dir), c.runtime.keep_backups))
    }

    pub fn draw(&mut self, f: &mut Frame, area: Rect) {
        let [head, body] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(area);
        let n = self.changed.len();
        let stage = match &self.stage {
            Stage::Edit => String::new(),
            Stage::Verifying { .. } => " | apply: waiting for the service".into(),
            Stage::Result(m) => format!(" | {m}"),
        };
        let line = format!(
            " Settings: {n} changed {} | {} secret(s) | {}{stage}",
            if n == 1 { "key" } else { "keys" },
            self.secrets.len(),
            self.path.display()
        );
        f.render_widget(
            Paragraph::new(line).style(Style::new().add_modifier(Modifier::BOLD)),
            head,
        );
        let Some(_) = &self.draft else {
            let text = format!(
                "{}\n\nfix the file by hand, then start the TUI again.\n\n{}",
                self.load_error.as_deref().unwrap_or_default(),
                self.raw
            );
            f.render_widget(
                Paragraph::new(text).wrap(Wrap { trim: false }).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(" config file "),
                ),
                body,
            );
            return;
        };
        let [g, k, h] = if body.width >= 100 {
            Layout::horizontal([
                Constraint::Percentage(24),
                Constraint::Percentage(42),
                Constraint::Percentage(34),
            ])
            .areas(body)
        } else {
            let [top, help] =
                Layout::vertical([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .areas(body);
            let [g, k] =
                Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                    .areas(top);
            [g, k, help]
        };
        self.areas = [g, k, h];
        let hl = Style::new()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD);
        let focus = |on: bool| {
            if on {
                Style::new().fg(Color::Cyan)
            } else {
                Style::new()
            }
        };
        let items: Vec<ListItem> = self
            .groups
            .iter()
            .map(|g| ListItem::new(g.title.clone()))
            .collect();
        let mut st = ListState::default().with_selected(Some(self.group));
        f.render_stateful_widget(
            List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(focus(self.focus == 0))
                        .title(" groups "),
                )
                .highlight_style(hl),
            g,
            &mut st,
        );
        let changed: HashSet<&str> = self.changed.iter().map(String::as_str).collect();
        let rows: Vec<ListItem> = self
            .groups
            .get(self.group)
            .map(|grp| {
                grp.keys
                    .iter()
                    .map(|(p, m)| {
                        let name = m.path.rsplit('.').next().unwrap_or_default();
                        let d = self.draft.as_ref();
                        let text = match d.and_then(|d| d.effective(p)) {
                            Some((v, Origin::File)) => show(&v),
                            Some((v, Origin::RouterDefaults)) => {
                                format!("{} (router_defaults)", show(&v))
                            }
                            Some((v, Origin::Default)) => format!("{} (default)", show(&v)),
                            None => "(not set)".into(),
                        };
                        let mark = if changed.contains(path_text(p).as_str()) {
                            let was = d
                                .and_then(|d| d.get_original(p))
                                .and_then(|i| i.into_value().ok())
                                .map_or("default".into(), |v| show(&v));
                            format!("  * was {was}")
                        } else {
                            String::new()
                        };
                        let ro = if m.read_only { "  (read-only)" } else { "" };
                        ListItem::new(format!("{name} = {text}{mark}{ro}"))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let mut st = ListState::default().with_selected((self.focus == 1).then_some(self.key));
        f.render_stateful_widget(
            List::new(rows)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(focus(self.focus == 1))
                        .title(" keys "),
                )
                .highlight_style(hl),
            k,
            &mut st,
        );
        f.render_widget(
            Paragraph::new(self.right_panel())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.conflict.is_some() {
                            " the file on disk changed "
                        } else if self.current_preview().is_some() {
                            " preview ([ ] scroll) "
                        } else {
                            " help "
                        }),
                ),
            h,
        );
        self.draw_popup(f, k);
    }

    fn right_panel(&self) -> Vec<Line<'static>> {
        let mut top: Vec<Line<'static>> = Vec::new();
        if let Stage::Result(m) = &self.stage {
            top.push(Line::styled(
                m.clone(),
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
            top.push(Line::from(""));
        }
        if let Some((_, diff)) = &self.conflict {
            top.push(Line::from(
                "the changes on disk since the start of the edit:",
            ));
            for (c, l) in diff.iter().filter(|(c, _)| *c != ' ') {
                let color = if *c == '+' { Color::Green } else { Color::Red };
                top.push(Line::styled(format!("{c} {l}"), Style::new().fg(color)));
            }
            top.push(Line::from(""));
            top.push(Line::from(
                "D: drop your edits and start from the file on disk",
            ));
            return top;
        }
        top.extend(self.panel_body());
        top
    }

    fn panel_body(&self) -> Vec<Line<'static>> {
        if let Some(s) = self.current_preview() {
            // The warnings first: a long list of circuit lines must not hide them.
            let mut out: Vec<Line> = self
                .all_warnings(s)
                .into_iter()
                .map(|w| Line::styled(format!("warning: {w}"), Style::new().fg(Color::Yellow)))
                .collect();
            out.extend(
                self.draft
                    .as_ref()
                    .map(|d| d.diff_lines())
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|(c, _)| *c != ' ')
                    .map(|(c, l)| {
                        let color = if c == '+' { Color::Green } else { Color::Red };
                        Line::styled(format!("{c} {l}"), Style::new().fg(color))
                    }),
            );
            if !self.secrets.is_empty() {
                out.push(Line::from(format!(
                    "secret files to write: {}",
                    self.secrets
                        .iter()
                        .map(|p| p.path.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
            }
            out.push(Line::from(""));
            match &s.result {
                Preview::Ran(r) => out.extend(r.text.lines().map(|l| Line::from(l.to_string()))),
                Preview::Skipped(why) => out.push(Line::from(format!("no output preview: {why}"))),
                Preview::Failed(why) => out.push(Line::styled(
                    format!("preview failed: {why}"),
                    Style::new().fg(Color::Red),
                )),
            }
            out.push(Line::from(""));
            out.push(Line::from("A apply (asks first)"));
            return out;
        }
        let Some((p, m)) = self.current() else {
            return vec![Line::from("no key")];
        };
        let mut out: Vec<Line> = m.help.lines().map(|l| Line::from(l.to_string())).collect();
        out.push(Line::from(""));
        out.push(Line::from(format!("key: {}", path_text(p))));
        out.push(Line::from(format!(
            "default: {}",
            m.default.as_deref().unwrap_or("none")
        )));
        let kind = match &m.kind {
            Kind::Bool => "true or false (Space)".to_string(),
            Kind::Int => "whole number".into(),
            Kind::Float => "number".into(),
            Kind::Text => "text".into(),
            Kind::Choice(c) => format!("one of: {}", c.join(" | ")),
            Kind::List(_) => "list (Enter: a add, d delete, K/J move)".into(),
        };
        out.push(Line::from(format!("type: {kind}")));
        if m.optional {
            out.push(Line::from("optional: r removes it"));
        }
        if m.read_only {
            out.push(Line::styled(
                "read-only here",
                Style::new().fg(Color::Yellow),
            ));
        }
        if m.path.ends_with("password_file") || m.path.ends_with("bot_token_file") {
            out.push(Line::from("P: set a new secret (hidden)"));
        }
        out
    }

    /// The warnings of validate and of the preview.
    fn all_warnings(&self, s: &Shown) -> Vec<String> {
        let mut w = s.warnings.clone();
        if let Preview::Ran(r) = &s.result {
            w.extend(r.warnings.iter().cloned());
        }
        w
    }

    fn router_name(&self, r: usize) -> String {
        self.draft
            .as_ref()
            .and_then(|d| d.get(&concrete("routers.*.name", &[r])))
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_else(|| format!("#{r}"))
    }

    fn draw_popup(&self, f: &mut Frame, area: Rect) {
        let popup = |f: &mut Frame, title: &str, lines: Vec<Line>, h: u16| {
            let h = h.min(area.height);
            let r = Rect::new(area.x, area.y + (area.height - h) / 2, area.width, h);
            f.render_widget(Clear, r);
            f.render_widget(
                Paragraph::new(lines).wrap(Wrap { trim: false }).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" {title} ")),
                ),
                r,
            );
        };
        match &self.input {
            Input::None => {}
            Input::Field { path, buf, .. } => popup(
                f,
                &path_text(path),
                vec![
                    Line::from(format!("{buf}_")),
                    Line::from("Enter save, Esc cancel"),
                ],
                4,
            ),
            Input::Dropdown {
                path, choices, sel, ..
            } => {
                let lines = choices
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == *sel {
                            Line::styled(
                                format!("> {c}"),
                                Style::new().add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Line::from(format!("  {c}"))
                        }
                    })
                    .collect();
                popup(f, &path_text(path), lines, choices.len() as u16 + 2);
            }
            Input::List { path, sel, .. } => {
                let items: Vec<String> = self
                    .draft
                    .as_ref()
                    .and_then(|d| d.effective(path))
                    .and_then(|(v, _)| v.as_array().map(|a| a.iter().map(show).collect()))
                    .unwrap_or_default();
                let mut lines: Vec<Line> = items
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        if i == *sel {
                            Line::styled(
                                format!("> {x}"),
                                Style::new().add_modifier(Modifier::BOLD),
                            )
                        } else {
                            Line::from(format!("  {x}"))
                        }
                    })
                    .collect();
                lines.push(Line::from("a add, d delete, K up, J down, Esc close"));
                popup(f, &path_text(path), lines, items.len() as u16 + 3);
            }
            Input::Secret { path, value } => popup(
                f,
                "new secret (hidden)",
                vec![
                    Line::from(format!("file: {}", path.display())),
                    Line::from("*".repeat(value.len())),
                    Line::from("Enter keep for apply, Esc cancel"),
                ],
                5,
            ),
            Input::AddServer { buf, .. } => popup(
                f,
                "add DHCP server",
                vec![
                    Line::from(format!("name: {buf}_")),
                    Line::from("Enter add, Esc cancel"),
                ],
                4,
            ),
            Input::AddRouter {
                step,
                fields,
                buf,
                pw,
            } => {
                let labels = ["name", "address", "username"];
                let mut lines: Vec<Line> = (0..3)
                    .map(|i| {
                        let v = if i < *step {
                            fields[i].clone()
                        } else if i == *step {
                            format!("{buf}_")
                        } else {
                            String::new()
                        };
                        Line::from(format!("{}: {v}", labels[i]))
                    })
                    .collect();
                lines.push(Line::from(format!(
                    "password (hidden): {}",
                    if *step == 3 {
                        "*".repeat(pw.len()) + "_"
                    } else {
                        String::new()
                    }
                )));
                lines.push(Line::from("Enter next, Esc cancel"));
                popup(f, "add router", lines, 7);
            }
        }
        match &self.modal {
            None => {}
            Some(VModal::ConfirmApply) => {
                let n = self.changed.len();
                let mut lines = vec![Line::from(format!(
                    "{n} changed {}",
                    if n == 1 { "key" } else { "keys" }
                ))];
                if let Some(s) = self.current_preview() {
                    match &s.result {
                        Preview::Ran(r) => {
                            let c = r.counts;
                            lines.push(Line::from(format!(
                                "circuits +{} -{} ~{}; nodes +{} -{} ~{}",
                                c[0], c[1], c[2], c[3], c[4], c[5]
                            )));
                            if !r.held.is_empty() {
                                lines.push(Line::from(format!(
                                    "held: {} circuits, removed after one held cycle",
                                    r.held.len()
                                )));
                            }
                        }
                        Preview::Skipped(why) => {
                            lines.push(Line::from(format!("no output change: {why}")))
                        }
                        Preview::Failed(_) => {}
                    }
                    for w in self.all_warnings(s) {
                        lines.push(Line::styled(
                            format!("warning: {w}"),
                            Style::new().fg(Color::Yellow),
                        ));
                    }
                }
                for p in &self.secrets {
                    lines.push(Line::from(format!("secret file: {}", p.path.display())));
                }
                lines.push(Line::from(
                    "y apply (write the file, then SIGHUP), another key cancels",
                ));
                let h = lines.len() as u16 + 2;
                popup(f, "apply", lines, h);
            }
            Some(VModal::RemoveRouter(r)) => popup(
                f,
                &format!("remove router {}", self.router_name(*r)),
                vec![
                    Line::from(format!(
                        "remove router {} and all its settings",
                        self.router_name(*r)
                    )),
                    Line::from(format!(
                        "the password file {} stays: delete it by hand",
                        self.router_password_file(*r)
                    )),
                    Line::from("y remove, another key cancels"),
                ],
                5,
            ),
            Some(VModal::RemoveServer(r, j)) => {
                let sname = self
                    .draft
                    .as_ref()
                    .and_then(|d| d.get(&concrete("routers.*.dhcp.servers.*.name", &[*r, *j])))
                    .and_then(|v| v.as_str().map(str::to_string))
                    .unwrap_or_else(|| format!("#{j}"));
                popup(
                    f,
                    "remove DHCP server",
                    vec![
                        Line::from(format!(
                            "remove DHCP server {sname} of router {}",
                            self.router_name(*r)
                        )),
                        Line::from("y remove, another key cancels"),
                    ],
                    4,
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    const FILE: &str = "[runtime]\nmode = \"shadow\" # shadow | apply\n\n[[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/etc/lqosync3/secrets/R1.password\"\n";

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn view(read_only: bool) -> SettingsView {
        SettingsView::new(PathBuf::from("/tmp/x.toml"), FILE, read_only)
    }

    fn screen(v: &mut SettingsView, w: u16, h: u16) -> String {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| v.draw(f, f.area())).unwrap();
        let buf = t.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n")
            .collect()
    }

    fn typing(v: &mut SettingsView, s: &str) {
        for c in s.chars() {
            v.on_key(key(KeyCode::Char(c)));
        }
    }

    fn done(v: &mut SettingsView, result: Preview) {
        let SAction::Preview(job) = v.on_key(key(KeyCode::Char('v'))) else {
            panic!("no preview job")
        };
        v.preview_done(PreviewDone {
            id: job.id,
            result,
            warnings: vec![],
        });
    }

    fn ran() -> Preview {
        Preview::Ran(super::super::preview::Ran {
            text: "circuits: +0 -0 ~1; nodes: +0 -0 ~0\n".into(),
            counts: [0, 0, 1, 0, 0, 0],
            warnings: vec![],
            held: vec![],
        })
    }

    #[test]
    fn settings_tab_renders_groups_keys_and_help() {
        let mut v = view(false);
        assert!(v.select("runtime.mode"));
        let s = screen(&mut v, 140, 40);
        for want in [
            "runtime",
            "router R1",
            "mode",
            "\"shadow\"",
            "shadow | apply",
            "default",
        ] {
            assert!(s.contains(want), "{want}\n{s}");
        }
    }

    #[test]
    fn narrow_terminal_puts_help_under_the_keys() {
        let mut v = view(false);
        v.select("runtime.mode");
        screen(&mut v, 80, 40);
        let (keys, help) = (v.areas[1], v.areas[2]);
        assert!(help.y > keys.y, "{keys:?} {help:?}");
    }

    #[test]
    fn space_toggles_a_bool() {
        let mut v = view(false);
        assert!(v.select("safety.enabled"));
        v.on_key(key(KeyCode::Char(' ')));
        assert!(v.draft.as_ref().unwrap().text().contains("enabled = false"));
        assert_eq!(v.draft.as_ref().unwrap().changed(), ["safety.enabled"]);
        assert!(screen(&mut v, 140, 40).contains("1 changed"));
    }

    #[test]
    fn enter_on_an_enum_shows_its_choices() {
        let mut v = view(false);
        v.select("network.mode");
        v.on_key(key(KeyCode::Enter));
        let s = screen(&mut v, 140, 40);
        for c in [
            "grouped_router_root",
            "flat_router_root",
            "flat_global_root",
            "no_parent",
            "root_promotion",
            "flat_promotion",
        ] {
            assert!(s.contains(c), "{c}\n{s}");
        }
        v.on_key(key(KeyCode::Down));
        v.on_key(key(KeyCode::Enter));
        assert!(
            v.draft
                .as_ref()
                .unwrap()
                .text()
                .contains("mode = \"flat_router_root\"")
        );
    }

    #[test]
    fn a_bad_number_shows_the_key_error() {
        let mut v = view(false);
        v.select("runtime.id_length");
        v.on_key(key(KeyCode::Enter));
        for _ in 0..3 {
            v.on_key(key(KeyCode::Backspace));
        }
        typing(&mut v, "abc");
        v.on_key(key(KeyCode::Enter));
        assert!(
            v.message.as_deref().unwrap().contains("runtime.id_length"),
            "{:?}",
            v.message
        );
        v.on_key(key(KeyCode::Enter));
        typing(&mut v, "99");
        v.on_key(key(KeyCode::Enter));
        // 899 or 99: out of range; the value stays in the draft and the error names the key.
        assert!(
            v.message.as_deref().unwrap().contains("id_length"),
            "{:?}",
            v.message
        );
        assert!(v.draft.as_ref().unwrap().text().contains("id_length"));
    }

    #[test]
    fn a_typed_password_is_never_in_the_buffer() {
        let mut v = view(false);
        v.select("routers.0.password_file");
        v.on_key(key(KeyCode::Char('P')));
        typing(&mut v, "S3cret!");
        let s = screen(&mut v, 140, 40);
        assert!(!s.contains("S3cret") && s.contains("*******"), "{s}");
        v.on_key(key(KeyCode::Enter));
        let s = screen(&mut v, 140, 40);
        let d = v.draft.as_ref().unwrap();
        assert!(!s.contains("S3cret") && !d.text().contains("S3cret"));
        assert!(!format!("{:?}", v.message).contains("S3cret"));
        assert!(!format!("{:?}", v.secrets).contains("S3cret"));
        assert_eq!(v.secrets.len(), 1);
        assert!(v.dirty());
    }

    #[test]
    fn apply_needs_a_preview_of_the_same_text() {
        let mut v = view(false);
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        assert_eq!(v.on_key(key(KeyCode::Char('A'))), SAction::None);
        assert!(v.message.as_deref().unwrap().contains("preview"));
        done(&mut v, ran());
        // A change after the preview needs a new preview.
        v.select("runtime.mode");
        v.on_key(key(KeyCode::Enter));
        v.on_key(key(KeyCode::Down));
        v.on_key(key(KeyCode::Enter));
        assert_eq!(v.on_key(key(KeyCode::Char('A'))), SAction::None);
    }

    #[test]
    fn apply_asks_for_a_confirmation() {
        let mut v = view(false);
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        done(&mut v, ran());
        assert_eq!(v.on_key(key(KeyCode::Char('A'))), SAction::None);
        let s = screen(&mut v, 140, 40);
        assert!(s.contains("1 changed key") && s.contains("y apply"), "{s}");
        assert_eq!(v.on_key(key(KeyCode::Char('y'))), SAction::Apply);
    }

    #[test]
    fn a_failed_preview_blocks_apply() {
        let mut v = view(false);
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        done(&mut v, Preview::Failed("bad network.json".into()));
        assert_eq!(v.on_key(key(KeyCode::Char('A'))), SAction::None);
        assert!(v.modal.is_none());
        assert!(screen(&mut v, 140, 40).contains("bad network.json"));
    }

    #[test]
    fn read_only_blocks_an_edit() {
        let mut v = view(true);
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        assert!(!v.dirty());
        assert!(v.message.as_deref().unwrap().contains("read-only"));
    }

    #[test]
    fn state_dir_cannot_be_edited() {
        let mut v = view(false);
        v.select("runtime.state_dir");
        v.on_key(key(KeyCode::Enter));
        assert!(!v.capturing() && !v.dirty());
        assert!(
            v.message
                .as_deref()
                .unwrap()
                .contains("copy the folder first")
        );
    }

    #[test]
    fn a_broken_config_file_shows_the_error() {
        let mut v = SettingsView::new(PathBuf::from("/tmp/x.toml"), "[runtime\nmode = 1", false);
        assert!(v.draft.is_none());
        let s = screen(&mut v, 120, 20);
        assert!(
            s.contains("not valid TOML") && s.contains("fix the file by hand"),
            "{s}"
        );
    }

    #[test]
    fn add_a_router_asks_for_a_masked_password() {
        let mut v = view(false);
        v.select("routers.0.name");
        v.on_key(key(KeyCode::Char('+')));
        for part in ["R2", "192.0.2.2", "admin"] {
            typing(&mut v, part);
            v.on_key(key(KeyCode::Enter));
        }
        typing(&mut v, "pw2");
        assert!(!screen(&mut v, 140, 40).contains("pw2"));
        v.on_key(key(KeyCode::Enter));
        let t = v.draft.as_ref().unwrap().text();
        assert!(
            t.contains("name = \"R2\"") && t.contains("/etc/lqosync3/secrets/R2.password"),
            "{t}"
        );
        assert_eq!(
            v.secrets[0].path,
            PathBuf::from("/etc/lqosync3/secrets/R2.password")
        );
    }

    fn ran_with(lines: usize, warnings: &[&str]) -> Preview {
        let mut text: String = (0..lines)
            .map(|i| format!("~ circuit c{i}: Download Min Mbps 1 -> 2\n"))
            .collect();
        text.push_str(&format!("circuits: +0 -0 ~{lines}; nodes: +0 -0 ~0\n"));
        Preview::Ran(super::super::preview::Ran {
            text,
            counts: [0, 0, lines, 0, 0, 0],
            warnings: warnings.iter().map(|w| w.to_string()).collect(),
            held: vec![],
        })
    }

    #[test]
    fn warnings_come_first_and_the_confirm_lists_them() {
        let mut v = view(false);
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        done(&mut v, ran_with(500, &["W-apply-mode", "W-router-X"]));
        let s = screen(&mut v, 140, 40);
        assert!(
            s.contains("W-apply-mode") && s.contains("W-router-X"),
            "{s}"
        );
        v.on_key(key(KeyCode::Char('A')));
        let s = screen(&mut v, 140, 40);
        assert!(s.lines().filter(|l| l.contains("W-")).count() >= 4, "{s}");
        v.on_key(key(KeyCode::Esc));
        v.on_key(key(KeyCode::Char(']')));
        assert!(v.scroll > 0);
    }

    const RD: &str = "[router_defaults.pppoe]\nenabled = true\n[router_defaults.exclude]\nips = [\"10.0.0.0/8\"]\n\n";

    #[test]
    fn router_keys_use_the_router_defaults_of_the_file() {
        let mut v = SettingsView::new(PathBuf::from("/tmp/x.toml"), &format!("{RD}{FILE}"), false);
        assert!(v.select("routers.0.pppoe.enabled"));
        let s = screen(&mut v, 160, 40);
        assert!(s.contains("enabled = true (router_defaults)"), "{s}");
        assert!(v.select("routers.0.exclude.ips"));
        v.on_key(key(KeyCode::Enter));
        v.on_key(key(KeyCode::Char('a')));
        typing(&mut v, "10.1.0.0/16");
        v.on_key(key(KeyCode::Enter));
        let c = crate::config::Config::from_toml_str(&v.draft.as_ref().unwrap().text()).unwrap();
        assert_eq!(c.routers[0].exclude.ips, ["10.0.0.0/8", "10.1.0.0/16"]);
    }

    #[test]
    fn ctrl_c_in_a_secret_field_cancels_and_ctrl_keys_type_nothing() {
        let mut v = view(false);
        v.select("routers.0.password_file");
        v.on_key(key(KeyCode::Char('P')));
        typing(&mut v, "ab");
        v.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL));
        v.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
        v.on_key(key(KeyCode::Enter));
        assert_eq!(v.secrets[0].value.len(), 2);
        v.on_key(key(KeyCode::Char('P')));
        v.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert!(!v.capturing());
    }

    #[test]
    fn run_preview_allows_a_fix_of_an_invalid_current_file() {
        let dir = tempfile::tempdir().unwrap();
        let lqos = dir.path().join("lqos");
        std::fs::create_dir_all(&lqos).unwrap();
        let pw = dir.path().join("R1.password");
        let good = format!(
            "[runtime]\nstate_dir = \"{}\"\n[libreqos]\ntarget = \"v2_pair\"\ndirectory = \"{}\"\n[[routers]]\nname = \"R1\"\naddress = \"a\"\nusername = \"u\"\npassword_file = \"{}\"\n[routers.pppoe]\nenabled = true\n",
            dir.path().join("state").display(),
            lqos.display(),
            pw.display()
        );
        let bad = good.replace("[runtime]", "[runtime]\nmode = \"x\"");
        let job = PreviewJob {
            id: 1,
            old_text: bad,
            new_text: good,
            typed: HashSet::from([pw]),
            changed: vec!["runtime.mode".into()],
            loaded_digest: None,
        };
        let d = run_preview(job);
        assert!(matches!(d.result, Preview::Skipped(_)), "{:?}", d.result);
        assert!(
            d.warnings.iter().any(|w| w.contains("current file")),
            "{:?}",
            d.warnings
        );
    }

    struct Disk {
        _dir: tempfile::TempDir,
        path: PathBuf,
        state: PathBuf,
    }

    fn disk() -> (Disk, SettingsView) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lqosync3.toml");
        std::fs::write(&path, FILE).unwrap();
        let state = dir.path().join("state");
        std::fs::create_dir_all(&state).unwrap();
        let v = SettingsView::new(path.clone(), FILE, false);
        (
            Disk {
                _dir: dir,
                path,
                state,
            },
            v,
        )
    }

    #[test]
    fn a_changed_file_shows_the_disk_diff_and_can_restart() {
        let (d, mut v) = disk();
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        let other = FILE.replace("mode = \"shadow\"", "mode = \"apply\"");
        std::fs::write(&d.path, &other).unwrap();
        assert!(v.do_apply(&d.state, 5, 1_000).is_err());
        let s = screen(&mut v, 160, 40);
        assert!(
            s.contains("+ mode = \"apply\"") && s.contains("- mode = \"shadow\""),
            "{s}"
        );
        v.on_key(key(KeyCode::Char('D')));
        assert_eq!(v.draft.as_ref().unwrap().text(), other);
        assert!(!v.dirty());
    }

    #[test]
    fn a_rejected_apply_says_the_new_file_is_on_disk() {
        let (d, mut v) = disk();
        v.select("safety.enabled");
        v.on_key(key(KeyCode::Char(' ')));
        let digest = v.do_apply(&d.state, 5, 1_000).unwrap();
        let sent = crate::tui::dashboard::Sent {
            kind: crate::tui::dashboard::Kind::Reload,
            at: 1_000,
            pid: 7,
            seq: 2,
        };
        v.reload_sent(Ok(sent), digest);
        let st = serde_json::json!({"pid": 7, "trigger": "reload", "updated": 1_001, "config_digest": "old", "config_error": "bad key", "cycle_seq": 2});
        v.on_status(Some(&st), 1_001, 120);
        let s = screen(&mut v, 200, 40);
        assert!(
            s.contains("new file is on disk") && s.contains("000000001000.toml"),
            "{s}"
        );
    }

    #[test]
    fn a_secret_for_a_removed_router_is_dropped() {
        let mut v = view(false);
        v.select("routers.0.name");
        v.on_key(key(KeyCode::Char('+')));
        for part in ["R2", "192.0.2.2", "admin", "pw2"] {
            typing(&mut v, part);
            v.on_key(key(KeyCode::Enter));
        }
        assert_eq!(v.secrets.len(), 1);
        v.on_key(key(KeyCode::Char('u')));
        assert!(v.secrets.is_empty());
    }

    #[test]
    fn the_remove_dialog_names_the_router() {
        let mut v = view(false);
        v.select("routers.0.pppoe.enabled");
        v.on_key(key(KeyCode::Char('-')));
        assert!(screen(&mut v, 160, 40).contains("remove router R1"));
    }

    #[test]
    fn run_preview_skips_output_free_changes() {
        let dir = tempfile::tempdir().unwrap();
        let lqos = dir.path().join("lqos");
        std::fs::create_dir_all(&lqos).unwrap();
        let pw = dir.path().join("R1.password");
        let old = format!(
            "[runtime]\nstate_dir = \"{}\"\n[libreqos]\ntarget = \"v2_pair\"\ndirectory = \"{}\"\n[[routers]]\nname = \"R1\"\naddress = \"a\"\nusername = \"u\"\npassword_file = \"{}\"\n[routers.pppoe]\nenabled = true\n",
            dir.path().join("state").display(),
            lqos.display(),
            pw.display()
        );
        let new = old.replace("[libreqos]", "[tui]\nosc52 = false\n[libreqos]");
        let job = PreviewJob {
            id: 1,
            old_text: old.clone(),
            new_text: new,
            typed: HashSet::from([pw.clone()]),
            changed: vec!["tui.osc52".into()],
            loaded_digest: None,
        };
        let d = run_preview(job);
        assert!(matches!(d.result, Preview::Skipped(_)), "{:?}", d.result);
        let job = PreviewJob {
            id: 2,
            old_text: old.clone(),
            new_text: old.replace("mode = \"x\"", ""),
            typed: HashSet::new(),
            changed: vec!["runtime.mode".into()],
            loaded_digest: Some("other".into()),
        };
        let d = run_preview(job);
        assert!(
            matches!(&d.result, Preview::Failed(e) if e.contains("R1")),
            "{:?}",
            d.result
        );
    }
}
