//! The draft: the user config file as a `toml_edit` document, so comments and key order stay.

use super::meta;
use anyhow::{Context, anyhow, bail, ensure};
use std::collections::{BTreeMap, HashSet};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Item, Table, TableLike, Value};

/// One step of a key path: a table key, or a list index (`[[routers]]` entry, array item).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Seg {
    Key(String),
    Index(usize),
}

pub type KeyPath = Vec<Seg>;

/// `routers.0.pppoe.enabled`.
pub fn path_text(p: &[Seg]) -> String {
    p.iter()
        .map(|s| match s {
            Seg::Key(k) => k.clone(),
            Seg::Index(i) => i.to_string(),
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// `routers.*.pppoe.enabled`: the path of the key metadata.
pub fn meta_path(p: &[Seg]) -> String {
    p.iter()
        .map(|s| match s {
            Seg::Key(k) => k.as_str(),
            Seg::Index(_) => "*",
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// A router name goes into the secret file name: no `/`, no `..`.
pub fn valid_router_name(n: &str) -> bool {
    (1..=64).contains(&n.len())
        && n != "."
        && n != ".."
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub struct Draft {
    original: String,
    orig_doc: DocumentMut,
    /// `cli::config_digest` of the file text at the start of the edit.
    pub start_digest: String,
    doc: DocumentMut,
    /// The tables that the editor made, as `path_text`.
    made: HashSet<String>,
    /// The text and `made` before each change.
    undo: Vec<(String, HashSet<String>)>,
}

/// The leaf values of a document, as `path_text` to value text, for `changed`.
fn leaves(t: &dyn TableLike, prefix: &str, out: &mut BTreeMap<String, String>) {
    for (k, item) in t.iter() {
        let p = if prefix.is_empty() {
            k.to_string()
        } else {
            format!("{prefix}.{k}")
        };
        match item {
            Item::Value(Value::InlineTable(it)) => leaves(it, &p, out),
            Item::Value(v) => {
                out.insert(p, v.clone().decorated("", "").to_string());
            }
            Item::Table(t) => leaves(t, &p, out),
            Item::ArrayOfTables(a) => {
                for (i, t) in a.iter().enumerate() {
                    leaves(t, &format!("{p}.{i}"), out);
                }
            }
            Item::None => {}
        }
    }
}

impl Draft {
    pub fn new(text: &str) -> anyhow::Result<Draft> {
        let doc: DocumentMut = text.parse().context("the config file is not valid TOML")?;
        Ok(Draft {
            original: text.to_string(),
            orig_doc: doc.clone(),
            start_digest: crate::cli::config_digest(text.as_bytes()),
            doc,
            made: HashSet::new(),
            undo: Vec::new(),
        })
    }

    pub fn text(&self) -> String {
        self.doc.to_string()
    }

    pub fn original(&self) -> &str {
        &self.original
    }

    fn save(&mut self) {
        self.undo.push((self.text(), self.made.clone()));
    }

    /// Go back one change. `false` if there is no change.
    pub fn undo(&mut self) -> bool {
        let Some((text, made)) = self.undo.pop() else {
            return false;
        };
        self.doc = text.parse().expect("an undo text was valid TOML");
        self.made = made;
        true
    }

    /// The table at `p`. With `create`, a missing table is made as a standard table
    /// (`[a.b]`, not an inline table); a made parent that has only subtables is implicit.
    fn table_mut(&mut self, p: &[Seg], create: bool) -> anyhow::Result<&mut dyn TableLike> {
        let mut made = Vec::new();
        let mut cur: &mut dyn TableLike = self.doc.as_table_mut();
        let mut i = 0;
        while i < p.len() {
            let Seg::Key(k) = &p[i] else {
                bail!("{}: an index without a list", path_text(&p[..=i]));
            };
            if cur.get(k).is_none() {
                ensure!(create, "{} does not exist", path_text(&p[..=i]));
                let mut t = Table::new();
                // A parent of a subtable prints no header of its own.
                t.set_implicit(i + 1 < p.len());
                cur.insert(k, Item::Table(t));
                made.push(path_text(&p[..=i]));
            }
            let item = cur.get_mut(k).expect("the key exists now");
            cur = match item {
                Item::ArrayOfTables(a) => {
                    let Some(Seg::Index(n)) = p.get(i + 1) else {
                        bail!("{}: a list needs an index", path_text(&p[..=i]));
                    };
                    i += 1;
                    let len = a.len();
                    a.get_mut(*n).ok_or_else(|| {
                        anyhow!("{}: entry {n} of {len} does not exist", path_text(&p[..i]))
                    })?
                }
                other => other
                    .as_table_like_mut()
                    .ok_or_else(|| anyhow!("{} is not a table", path_text(&p[..=i])))?,
            };
            i += 1;
        }
        self.made.extend(made);
        Ok(cur)
    }

    fn split(p: &[Seg]) -> anyhow::Result<(&[Seg], &str)> {
        match p.split_last() {
            Some((Seg::Key(k), parent)) => Ok((parent, k.as_str())),
            _ => bail!("{} is not a key", path_text(p)),
        }
    }

    /// The value in the file. `None`: the key is not in the file (the default applies).
    pub fn get(&self, p: &[Seg]) -> Option<Item> {
        get_in(&self.doc, p)
    }

    /// The value in the file at the start of the edit.
    pub fn get_original(&self, p: &[Seg]) -> Option<Item> {
        get_in(&self.orig_doc, p)
    }

    /// The number of `[[routers]]` entries, and of the DHCP servers of a router.
    pub fn count(&self, p: &[Seg]) -> usize {
        self.get(p)
            .and_then(|i| i.as_array_of_tables().map(|a| a.len()))
            .unwrap_or(0)
    }
}

fn get_in(doc: &DocumentMut, p: &[Seg]) -> Option<Item> {
    {
        let mut cur: &dyn TableLike = doc.as_table();
        let mut it = p.iter().peekable();
        while let Some(s) = it.next() {
            let Seg::Key(k) = s else { return None };
            let item = cur.get(k)?;
            if it.peek().is_none() {
                return Some(item.clone());
            }
            cur = match item {
                Item::ArrayOfTables(a) => match it.next()? {
                    Seg::Index(n) => a.get(*n)?,
                    Seg::Key(_) => return None,
                },
                other => other.as_table_like()?,
            };
        }
        None
    }
}

impl Draft {
    /// Set a key. An existing value keeps its decor (the trailing comment).
    pub fn set(&mut self, p: &[Seg], mut v: Value) -> anyhow::Result<()> {
        let (parent, key) = Self::split(p)?;
        let before = (self.text(), self.made.clone());
        let t = match self.table_mut(parent, true) {
            Ok(t) => t,
            Err(e) => {
                (self.doc, self.made) = (before.0.parse().expect("valid"), before.1);
                return Err(e);
            }
        };
        if let Some(Item::Value(old)) = t.get(key) {
            *v.decor_mut() = old.decor().clone();
        }
        t.insert(key, Item::Value(v));
        self.undo.push(before);
        Ok(())
    }

    /// Remove a key from the file, so the default applies. A table that the editor made
    /// and that is now empty is removed too.
    pub fn reset(&mut self, p: &[Seg]) -> anyhow::Result<()> {
        let (parent, key) = Self::split(p)?;
        if self.get(p).is_none() {
            return Ok(());
        }
        self.save();
        self.table_mut(parent, false)?.remove(key);
        let mut end = parent.len();
        while end > 0 {
            let path = path_text(&parent[..end]);
            let Some(Seg::Key(k)) = parent.get(end - 1) else {
                break;
            };
            let empty = self
                .get(&parent[..end])
                .and_then(|i| i.as_table_like().map(|t| t.is_empty()))
                .unwrap_or(false);
            if !(empty && self.made.contains(&path)) {
                break;
            }
            let k = k.clone();
            self.table_mut(&parent[..end - 1], false)?.remove(&k);
            self.made.remove(&path);
            end -= 1;
        }
        Ok(())
    }

    fn routers(&mut self) -> &mut ArrayOfTables {
        if !self.doc.contains_key("routers") {
            self.doc
                .insert("routers", Item::ArrayOfTables(ArrayOfTables::new()));
        }
        self.doc["routers"]
            .as_array_of_tables_mut()
            .expect("routers is a list of [[routers]] tables")
    }

    /// Add a `[[routers]]` entry. Returns its index.
    pub fn add_router(
        &mut self,
        name: &str,
        address: &str,
        username: &str,
        password_file: &str,
    ) -> anyhow::Result<usize> {
        ensure!(
            valid_router_name(name),
            "router name \"{name}\": use 1 to 64 of A-Z a-z 0-9 . _ - (the name is in the password file name)"
        );
        self.save();
        let mut t = Table::new();
        for (k, v) in [
            ("name", name),
            ("address", address),
            ("username", username),
            ("password_file", password_file),
        ] {
            t.insert(k, toml_edit::value(v));
        }
        let r = self.routers();
        r.push(t);
        Ok(r.len() - 1)
    }

    pub fn remove_router(&mut self, i: usize) -> anyhow::Result<()> {
        ensure!(i < self.routers().len(), "router {i} does not exist");
        self.save();
        self.routers().remove(i);
        Ok(())
    }

    fn servers(&mut self, router: usize) -> anyhow::Result<&mut ArrayOfTables> {
        let path = vec![
            Seg::Key("routers".into()),
            Seg::Index(router),
            Seg::Key("dhcp".into()),
        ];
        let dhcp = self.table_mut(&path, true)?;
        if dhcp.get("servers").is_none() {
            dhcp.insert("servers", Item::ArrayOfTables(ArrayOfTables::new()));
        }
        dhcp.get_mut("servers")
            .and_then(Item::as_array_of_tables_mut)
            .ok_or_else(|| anyhow!("routers.{router}.dhcp.servers is not a list of tables"))
    }

    /// Add a `[[routers.dhcp.servers]]` entry to a router. Returns its index.
    pub fn add_server(&mut self, router: usize, name: &str) -> anyhow::Result<usize> {
        ensure!(!name.trim().is_empty(), "a DHCP server needs a name");
        self.save();
        let mut t = Table::new();
        t.insert("name", toml_edit::value(name));
        let s = self.servers(router)?;
        s.push(t);
        Ok(s.len() - 1)
    }

    pub fn remove_server(&mut self, router: usize, i: usize) -> anyhow::Result<()> {
        self.save();
        let s = self.servers(router)?;
        ensure!(i < s.len(), "server {i} does not exist");
        s.remove(i);
        Ok(())
    }

    /// The array at `p`. A list that is not in the file starts from its default.
    fn array(&self, p: &[Seg]) -> anyhow::Result<Array> {
        match self.get(p) {
            Some(Item::Value(Value::Array(a))) => Ok(a),
            Some(_) => bail!("{} is not a list", path_text(p)),
            None => {
                let d = meta::find(&meta_path(p))
                    .and_then(|m| m.default.clone())
                    .unwrap_or_else(|| "[]".into());
                let doc: DocumentMut = format!("x = {d}\n").parse()?;
                doc["x"]
                    .as_array()
                    .cloned()
                    .ok_or_else(|| anyhow!("{}: the default is not a list", path_text(p)))
            }
        }
    }

    pub fn list_push(&mut self, p: &[Seg], v: Value) -> anyhow::Result<()> {
        let mut a = self.array(p)?;
        a.push_formatted(v.decorated(if a.is_empty() { "" } else { " " }, ""));
        self.set(p, Value::Array(a))
    }

    pub fn list_remove(&mut self, p: &[Seg], i: usize) -> anyhow::Result<()> {
        let mut a = self.array(p)?;
        ensure!(i < a.len(), "{}: item {i} does not exist", path_text(p));
        a.remove(i);
        a.fmt();
        self.set(p, Value::Array(a))
    }

    pub fn list_move(&mut self, p: &[Seg], from: usize, to: usize) -> anyhow::Result<()> {
        let mut a = self.array(p)?;
        ensure!(
            from < a.len() && to < a.len(),
            "{}: item does not exist",
            path_text(p)
        );
        let v = a.remove(from);
        a.insert_formatted(to, v);
        a.fmt();
        self.set(p, Value::Array(a))
    }

    /// The `path_text` of each leaf that is different from the original file.
    pub fn changed(&self) -> Vec<String> {
        let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
        leaves(self.orig_doc.as_table(), "", &mut a);
        leaves(self.doc.as_table(), "", &mut b);
        let keys: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
        keys.into_iter()
            .filter(|k| a.get(*k) != b.get(*k))
            .cloned()
            .collect()
    }

    /// A line diff of the original and the current text: `' '`, `'-'`, `'+'`.
    pub fn diff_lines(&self) -> Vec<(char, String)> {
        line_diff(&self.original, &self.text())
    }
}

/// An LCS line diff. The config files are small (a few hundred lines).
pub fn line_diff(old: &str, new: &str) -> Vec<(char, String)> {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let (n, m) = (a.len(), b.len());
    let mut l = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            l[i][j] = if a[i] == b[j] {
                l[i + 1][j + 1] + 1
            } else {
                l[i + 1][j].max(l[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            out.push((' ', a[i].to_string()));
            i += 1;
            j += 1;
        } else if j < m && (i == n || l[i][j + 1] >= l[i + 1][j]) {
            out.push(('+', b[j].to_string()));
            j += 1;
        } else {
            out.push(('-', a[i].to_string()));
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use toml_edit::Value;

    const FILE: &str = "# my config\n[runtime]\nmode = \"shadow\" # shadow | apply\n# the state\nstate_dir = \"/tmp/s\"\n\n[[routers]]\nname = \"R1\" # first\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/x\"\n";
    const MIN: &str = "[[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/x\"\n";
    const TWO: &str = "[[routers]]\nname = \"R1\"\naddress = \"192.0.2.1\"\nusername = \"u\"\npassword_file = \"/x\"\n\n[[routers]]\nname = \"R2\"\naddress = \"192.0.2.2\"\nusername = \"u\"\npassword_file = \"/y\"\n";

    fn p(s: &str) -> KeyPath {
        s.split('.')
            .map(|x| {
                x.parse::<usize>()
                    .map_or(Seg::Key(x.to_string()), Seg::Index)
            })
            .collect()
    }

    #[test]
    fn a_change_keeps_comments_and_key_order() {
        let mut d = Draft::new(FILE).unwrap();
        d.set(&p("runtime.mode"), Value::from("apply")).unwrap();
        let t = d.text();
        assert_eq!(
            t,
            FILE.replace(
                "mode = \"shadow\" # shadow | apply",
                "mode = \"apply\" # shadow | apply"
            )
        );
    }

    #[test]
    fn set_in_a_minimal_file_adds_the_table() {
        let mut d = Draft::new(MIN).unwrap();
        d.set(&p("runtime.scan_interval_idle"), Value::from(60))
            .unwrap();
        let t = d.text();
        assert!(t.lines().any(|l| l == "[runtime]"), "{t}");
        assert!(
            t.contains("scan_interval_idle = 60") && !t.contains("runtime = {"),
            "{t}"
        );
        let c = Config::from_toml_str(&t).unwrap();
        assert_eq!(
            (c.runtime.scan_interval_idle, c.routers[0].name.as_str()),
            (60, "R1")
        );
        d.set(&p("backstop_caps.dhcp.enabled"), Value::from(true))
            .unwrap();
        let t = d.text();
        assert!(
            t.contains("[backstop_caps.dhcp]") && !t.contains("[backstop_caps]\n"),
            "{t}"
        );
        assert!(
            Config::from_toml_str(&t)
                .unwrap()
                .backstop_caps
                .dhcp
                .enabled
        );
    }

    #[test]
    fn set_a_router_subtable_goes_under_its_entry() {
        let mut d = Draft::new(TWO).unwrap();
        d.set(&p("routers.0.pppoe.enabled"), Value::from(true))
            .unwrap();
        let t = d.text();
        let (a, b) = (
            t.find("[routers.pppoe]").unwrap(),
            t.rfind("[[routers]]").unwrap(),
        );
        assert!(a < b, "{t}");
        let c = Config::from_toml_str(&t).unwrap();
        assert!(c.routers[0].pppoe.enabled && !c.routers[1].pppoe.enabled);
    }

    #[test]
    fn reset_removes_the_key() {
        let mut d = Draft::new(FILE).unwrap();
        d.reset(&p("runtime.mode")).unwrap();
        assert!(!d.text().contains("mode ="));
        assert!(d.text().contains("[runtime]"));
        assert!(d.get(&p("runtime.mode")).is_none());
    }

    #[test]
    fn reset_removes_a_table_that_the_editor_made() {
        let mut d = Draft::new(MIN).unwrap();
        d.set(&p("runtime.scan_interval_idle"), Value::from(60))
            .unwrap();
        d.reset(&p("runtime.scan_interval_idle")).unwrap();
        assert_eq!(d.text(), MIN);
        let mut d = Draft::new(FILE).unwrap();
        d.reset(&p("runtime.mode")).unwrap();
        d.reset(&p("runtime.state_dir")).unwrap();
        assert!(
            d.text().contains("[runtime]"),
            "a table of the operator stays"
        );
    }

    #[test]
    fn add_and_remove_a_router() {
        let mut d = Draft::new(MIN).unwrap();
        let i = d
            .add_router("R2", "192.0.2.2", "u", "/etc/lqosync3/secrets/R2.password")
            .unwrap();
        assert_eq!(i, 1);
        let c = Config::from_toml_str(&d.text()).unwrap();
        assert_eq!(
            c.routers[1].password_file,
            "/etc/lqosync3/secrets/R2.password"
        );
        d.remove_router(0).unwrap();
        assert_eq!(
            Config::from_toml_str(&d.text()).unwrap().routers[0].name,
            "R2"
        );
    }

    #[test]
    fn a_bad_router_name_is_rejected() {
        let mut d = Draft::new(MIN).unwrap();
        for bad in ["../x", "a/b", "..", ".", "", "a b"] {
            assert!(d.add_router(bad, "192.0.2.2", "u", "/x").is_err(), "{bad}");
        }
        assert_eq!(d.text(), MIN);
    }

    #[test]
    fn add_and_remove_a_dhcp_server() {
        let mut d = Draft::new(TWO).unwrap();
        assert_eq!(d.add_server(1, "PLANS").unwrap(), 0);
        d.set(&p("routers.1.dhcp.servers.0.mode"), Value::from("per_plan"))
            .unwrap();
        let c = Config::from_toml_str(&d.text()).unwrap();
        assert_eq!(c.routers[1].dhcp.servers[0].name, "PLANS");
        assert!(c.routers[0].dhcp.servers.is_empty());
        d.remove_server(1, 0).unwrap();
        assert!(
            Config::from_toml_str(&d.text()).unwrap().routers[1]
                .dhcp
                .servers
                .is_empty()
        );
    }

    #[test]
    fn list_push_remove_move() {
        let mut d = Draft::new(MIN).unwrap();
        // A list that is not in the file starts from its default.
        d.list_push(
            &p("speed.dhcp_per_site_sources"),
            Value::from("lease_comment"),
        )
        .unwrap();
        let c = Config::from_toml_str(&d.text().replace("\"lease_comment\"", "\"server_comment\""))
            .unwrap();
        assert_eq!(c.speed.dhcp_per_site_sources.len(), 3);
        d.list_remove(&p("speed.dhcp_per_site_sources"), 2).unwrap();
        d.list_move(&p("speed.dhcp_per_site_sources"), 0, 1)
            .unwrap();
        let c = Config::from_toml_str(&d.text()).unwrap();
        assert_eq!(
            format!("{:?}", c.speed.dhcp_per_site_sources),
            "[Settings, ServerComment]"
        );
    }

    #[test]
    fn undo_goes_back_one_change() {
        let mut d = Draft::new(MIN).unwrap();
        d.set(&p("runtime.scan_interval_idle"), Value::from(60))
            .unwrap();
        d.set(&p("runtime.mode"), Value::from("apply")).unwrap();
        assert!(d.undo());
        assert!(!d.text().contains("mode") && d.text().contains("scan_interval_idle"));
        assert!(d.undo());
        assert_eq!(d.text(), MIN);
        assert!(!d.undo());
        // The `made` set came back too: a new set and reset removes the table again.
        d.set(&p("runtime.mode"), Value::from("apply")).unwrap();
        d.reset(&p("runtime.mode")).unwrap();
        assert_eq!(d.text(), MIN);
    }

    #[test]
    fn changed_counts_the_keys() {
        let mut d = Draft::new(FILE).unwrap();
        d.set(&p("runtime.mode"), Value::from("apply")).unwrap();
        d.set(&p("safety.enabled"), Value::from(false)).unwrap();
        d.reset(&p("runtime.state_dir")).unwrap();
        assert_eq!(
            d.changed(),
            ["runtime.mode", "runtime.state_dir", "safety.enabled"]
        );
    }

    #[test]
    fn diff_lines_show_old_and_new() {
        let mut d = Draft::new(FILE).unwrap();
        d.set(&p("runtime.mode"), Value::from("apply")).unwrap();
        let diff = d.diff_lines();
        assert!(diff.contains(&('-', "mode = \"shadow\" # shadow | apply".to_string())));
        assert!(diff.contains(&('+', "mode = \"apply\" # shadow | apply".to_string())));
        assert!(diff.contains(&(' ', "[runtime]".to_string())));
        assert_eq!(diff.iter().filter(|(c, _)| *c != ' ').count(), 2);
    }

    #[test]
    fn a_broken_file_is_an_error() {
        assert!(Draft::new("[runtime\nmode = 1").is_err());
    }
}
