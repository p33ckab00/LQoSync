//! The router inventory from the service cache `routers.json`. Reading it starts no router I/O.
use std::path::Path;
use web_contract::routers::{
    CACHE, RouterActivity, RouterEditOutcome, RouterEditRequest, RouterEvent, RouterInventory,
    VERSION,
};

/// The inventory part of a broker reply stays under this size. The reply limit is 64 KiB.
pub const BUDGET: usize = 60 * 1024;
/// More routers than an ISP inventory screen needs; the byte budget usually binds first.
pub const MAX_ROUTERS: usize = 200;

pub fn load(root: &Path) -> Option<RouterInventory> {
    let inventory: RouterInventory = crate::overview::read(root, CACHE)?;
    (inventory.version == VERSION && inventory.updated > 0).then(|| bound(inventory))
}

/// Keep the routers in config order and drop from the end until the reply fits.
pub fn bound(mut inventory: RouterInventory) -> RouterInventory {
    if inventory.routers.len() > MAX_ROUTERS {
        inventory.routers.truncate(MAX_ROUTERS);
        inventory.truncated = true;
    }
    while serde_json::to_vec(&inventory).map_or(usize::MAX, |b| b.len()) > BUDGET {
        if inventory.routers.pop().is_none() {
            break;
        }
        inventory.truncated = true;
    }
    inventory
}

/// The audit log that the core command appends (`router-audit.jsonl`, at most 1 MiB).
pub const AUDIT: &str = "router-audit.jsonl";
/// Events of one router in a reply, newest first.
pub const MAX_EVENTS: usize = 50;

pub fn activity(root: &Path, router_id: &str) -> RouterActivity {
    let bytes = std::fs::File::open(root.join(AUDIT))
        .and_then(|f| {
            use std::io::Read;
            let mut b = Vec::new();
            f.take(2 * 1024 * 1024).read_to_end(&mut b)?;
            Ok(b)
        })
        .unwrap_or_default();
    let mut events: Vec<RouterEvent> = bytes
        .split(|b| *b == b'\n')
        .rev()
        .filter_map(|line| serde_json::from_slice::<RouterEvent>(line).ok())
        .filter(|e| e.router_id == router_id)
        .take(MAX_EVENTS)
        .collect();
    // Keep the reply bounded even with long change lists.
    while serde_json::to_vec(&events).map_or(usize::MAX, |b| b.len()) > BUDGET {
        events.pop();
    }
    RouterActivity { events }
}

/// Run the core command once with a time limit. Its answer is the outcome; anything else is internal.
pub async fn run_editor(
    editor: &crate::broker::Editor,
    request: &RouterEditRequest,
) -> RouterEditOutcome {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fail = |m: &str| RouterEditOutcome {
        code: "internal".into(),
        message: m.into(),
        reply: None,
    };
    let Ok(mut child) = tokio::process::Command::new(&editor.bin)
        .arg("router-edit")
        .arg("--config")
        .arg(&editor.config)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
    else {
        return fail("The router editor could not start.");
    };
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        return fail("The router editor could not start.");
    };
    let body = serde_json::to_vec(request).unwrap_or_default();
    let run = async {
        stdin.write_all(&body).await?;
        drop(stdin);
        let mut out = Vec::new();
        stdout.take(64 * 1024).read_to_end(&mut out).await?;
        child.wait().await?;
        Ok::<_, std::io::Error>(out)
    };
    match tokio::time::timeout(std::time::Duration::from_secs(20), run).await {
        Ok(Ok(out)) => serde_json::from_slice(&out)
            .unwrap_or_else(|_| fail("The router editor answer is not valid.")),
        Ok(Err(_)) => fail("The router editor failed."),
        Err(_) => fail("The router editor did not answer in time."),
    }
}
