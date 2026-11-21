//! Bounded, read-only service logs. Only allowlisted units; the browser chooses no argument text.
use web_contract::journal::{JournalEntry, JournalPage, MAX_MESSAGE, PAGE, UNITS};

/// The reply part stays under this size; the oldest lines go first when it is larger.
const BUDGET: usize = 60 * 1024;

pub fn allowed(unit: &str) -> bool {
    UNITS.iter().any(|(u, _)| *u == unit)
}

/// The `journalctl` arguments for one page. `before_usec` asks for the lines before that time.
pub fn args(unit: &str, priority: u8, before_usec: Option<u64>) -> Vec<String> {
    let mut a = vec![
        "-u".to_string(),
        unit.to_string(),
        "-o".into(),
        "json".into(),
        "--no-pager".into(),
        "-n".into(),
        (PAGE + 1).to_string(),
        "-p".into(),
        format!("0..{}", priority.min(7)),
        "--output-fields=MESSAGE,PRIORITY,__REALTIME_TIMESTAMP".into(),
    ];
    if let Some(b) = before_usec {
        // --until includes its time, so ask for one microsecond earlier.
        let b = b.saturating_sub(1);
        a.push(format!("--until=@{}.{:06}", b / 1_000_000, b % 1_000_000));
    }
    a
}

/// One `journalctl -o json` line. A binary MESSAGE is an array of bytes in the JSON output.
fn entry(line: &str) -> Option<(JournalEntry, bool)> {
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    let at_usec = v["__REALTIME_TIMESTAMP"].as_str()?.parse().ok()?;
    let priority = v["PRIORITY"]
        .as_str()
        .and_then(|p| p.parse().ok())
        .unwrap_or(6);
    let text = match &v["MESSAGE"] {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(bytes) => String::from_utf8_lossy(
            &bytes
                .iter()
                .filter_map(|b| b.as_u64().map(|b| b as u8))
                .collect::<Vec<_>>(),
        )
        .into_owned(),
        _ => String::new(),
    };
    let cut = text.chars().count() > MAX_MESSAGE;
    let message: String = text.chars().take(MAX_MESSAGE).collect();
    Some((
        JournalEntry {
            at_usec,
            priority,
            message,
        },
        cut,
    ))
}

/// Build the page from the output. One line more than a page means that older lines exist.
pub fn page(unit: &str, output: &str) -> JournalPage {
    let mut cut = false;
    let mut entries: Vec<JournalEntry> = output
        .lines()
        .filter_map(entry)
        .map(|(e, c)| {
            cut |= c;
            e
        })
        .collect();
    let mut more = entries.len() > PAGE;
    if more {
        entries.drain(..entries.len() - PAGE);
    }
    let mut p = JournalPage {
        unit: unit.into(),
        entries,
        more,
        cut,
    };
    while serde_json::to_vec(&p).map_or(usize::MAX, |b| b.len()) > BUDGET && !p.entries.is_empty() {
        p.entries.remove(0);
        more = true;
        p.more = more;
    }
    p
}

/// Read one page. The program is `journalctl`, or `LQOSYNC_JOURNALCTL` for an isolated test.
pub async fn read(unit: &str, priority: u8, before_usec: Option<u64>) -> Option<JournalPage> {
    if !allowed(unit) {
        return None;
    }
    let program = std::env::var("LQOSYNC_JOURNALCTL").unwrap_or("journalctl".into());
    let run = tokio::process::Command::new(program)
        .args(args(unit, priority, before_usec))
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .output();
    let out = tokio::time::timeout(std::time::Duration::from_secs(10), run)
        .await
        .ok()?
        .ok()?;
    // journalctl exits 1 when a unit has no lines; the output is then empty, which is a page.
    let text = String::from_utf8_lossy(&out.stdout[..out.stdout.len().min(4 * 1024 * 1024)]);
    Some(page(unit, &text))
}
