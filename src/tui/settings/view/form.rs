//! Router form state. Password bytes stay in Secret until the existing apply path writes them.
use super::*;

pub(super) struct RouterForm {
    pub fields: [String; 6], // name, address, port, username, password-file, CA-file
    pub password: Secret,
    pub flags: [bool; 5], // TLS, verify, PPP, DHCP, Hotspot
    pub focus: usize,
    pub editing: bool,
    pub advanced: bool,
    defaults: crate::config::RouterCfg,
}
impl RouterForm {
    pub fn new(text: &str) -> Result<Self, String> {
        let table: toml::Table =
            toml::from_str(text).map_err(|e| format!("Cannot read current draft: {e}"))?;
        let names: Vec<&str> = table
            .get("routers")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|r| r.get("name").and_then(toml::Value::as_str))
            .collect();
        let mut n = names.len() + 1;
        while names.iter().any(|r| *r == format!("Router-{n}")) {
            n += 1;
        }
        let name = format!("Router-{n}");
        let probe = format!(
            "{text}\n[[routers]]\nname='{name}'\naddress='192.0.2.254'\nusername='lqos-reader'\npassword_file='/etc/lqosync3/secrets/{name}.password'\n"
        );
        let router = validate::parse(&probe)?
            .routers
            .pop()
            .ok_or("no router defaults")?;
        Ok(Self {
            fields: [
                name,
                String::new(),
                router.port.to_string(),
                router.username.clone(),
                router.password_file.clone(),
                router.tls_ca_file.clone(),
            ],
            password: Secret::new(String::new()),
            flags: [
                router.tls,
                router.tls_verify,
                router.pppoe.enabled,
                router.dhcp.enabled,
                router.hotspot.enabled,
            ],
            focus: 0,
            editing: true,
            advanced: false,
            defaults: router,
        })
    }
    pub fn field(&self, index: usize) -> String {
        match index {
            0..=3 => self.fields[index].clone(),
            4 => "*".repeat(self.password.len()),
            5 => self.fields[4].clone(),
            11 => self.fields[5].clone(),
            _ => String::new(),
        }
    }
    fn target(&mut self, index: usize) -> Option<&mut String> {
        match index {
            0..=3 => Some(&mut self.fields[index]),
            5 => Some(&mut self.fields[4]),
            11 => Some(&mut self.fields[5]),
            _ => None,
        }
    }
    pub fn edit_key(&mut self, k: KeyEvent) {
        if k.modifiers.contains(KeyModifiers::CONTROL) && k.code == KeyCode::Char('u') {
            if self.focus == 4 {
                self.password = Secret::new(String::new());
            } else if let Some(t) = self.target(self.focus) {
                t.clear();
            }
            return;
        }

        if k.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return;
        }
        match k.code {
            KeyCode::Char(c) if self.focus == 4 => self.password.push(c),
            KeyCode::Backspace if self.focus == 4 => self.password.pop(),
            KeyCode::Char(c) => {
                if let Some(t) = self.target(self.focus) {
                    t.push(c);
                }
            }
            KeyCode::Backspace => {
                if let Some(t) = self.target(self.focus) {
                    t.pop();
                }
            }
            _ => {}
        }
    }
    pub fn next(&mut self, back: bool) {
        loop {
            self.focus = (self.focus + if back { 14 } else { 1 }) % 15;
            if self.focus != 11 || self.advanced {
                break;
            }
        }
        self.editing = matches!(self.focus, 0..=5 | 11);
    }
    pub fn config(&self) -> Result<crate::config::RouterCfg, String> {
        if !valid_router_name(self.fields[0].trim()) {
            return Err("Name: use 1 to 64 letters, digits, . _ -".into());
        }
        if self.password.is_empty() {
            return Err("Password is required; it stays in memory until apply".into());
        }
        if !Path::new(self.fields[4].trim()).is_absolute() {
            return Err("Password file must be an absolute path".into());
        }
        let mut r = self.defaults.clone();
        r.name = self.fields[0].trim().into();
        r.address = self.fields[1].trim().into();
        r.port = self.fields[2]
            .trim()
            .parse()
            .map_err(|_| "API port must be 1 through 65535")?;
        if r.port == 0 {
            return Err("API port must be 1 through 65535".into());
        }
        r.username = self.fields[3].trim().into();
        r.password_file = self.fields[4].trim().into();
        r.tls_ca_file = self.fields[5].trim().into();
        r.tls = self.flags[0];
        r.tls_verify = self.flags[1];
        r.pppoe.enabled = self.flags[2];
        r.dhcp.enabled = self.flags[3];
        r.hotspot.enabled = self.flags[4];
        Ok(r)
    }
}
