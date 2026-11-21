//! Read-only service logs for the web Journal screen. The broker reads them with `journalctl`.
use serde::{Deserialize, Serialize};

/// The only units the Journal screen can read: (unit, label for the page).
pub const UNITS: [(&str, &str); 5] = [
    ("lqosync3.service", "LQoSync service"),
    ("lqosync3-web.service", "Web gateway"),
    ("lqosync3-web-broker.service", "Web broker"),
    ("lqosd.service", "LibreQoS daemon"),
    ("lqos_scheduler.service", "LibreQoS scheduler"),
];
/// Lines in one page; the page of older lines starts before the oldest line shown.
pub const PAGE: usize = 200;
/// A longer message is cut, and the page says so.
pub const MAX_MESSAGE: usize = 2000;

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Microseconds since the Unix epoch.
    pub at_usec: u64,
    /// syslog priority: 0 emergency … 3 error, 4 warning, 5 notice, 6 info, 7 debug.
    pub priority: u8,
    pub message: String,
}

#[derive(Clone, Default, Debug, PartialEq, Serialize, Deserialize)]
pub struct JournalPage {
    pub unit: String,
    /// Oldest first.
    pub entries: Vec<JournalEntry>,
    /// Older lines exist before the first entry.
    pub more: bool,
    /// One or more messages were cut to the length limit.
    pub cut: bool,
}
