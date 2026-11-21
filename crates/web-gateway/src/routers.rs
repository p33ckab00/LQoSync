//! The router inventory from the service cache `routers.json`. Reading it starts no router I/O.
use std::path::Path;
use web_contract::routers::{
    CACHE, RouterActivity, RouterEditOutcome, RouterEditRequest, RouterEvent, RouterInventory,
    RouterSummary, RouterTestResult, VERSION,
};

/// The inventory part of a broker reply stays under this size. The reply limit is 64 KiB.
pub const BUDGET: usize = 60 * 1024;
/// More routers than an ISP inventory screen needs; the byte budget usually binds first.
pub const MAX_ROUTERS: usize = 200;

fn read(root: &Path) -> Option<RouterInventory> {
    let inventory: RouterInventory = crate::overview::read(root, CACHE)?;
    (inventory.version == VERSION && inventory.updated > 0).then_some(inventory)
}

/// The inventory list: every router without its lists, bounded.
pub fn load(root: &Path) -> Option<RouterInventory> {
    let mut inventory = read(root)?;
    for r in &mut inventory.routers {
        r.lists = None;
    }
    Some(bound(inventory))
}

/// One router with its lists. Client overrides past the byte budget are dropped and flagged, so
/// the page can refuse to save a list that it did not get completely.
pub fn load_one(root: &Path, router_id: &str) -> Option<RouterSummary> {
    let mut r = read(root)?
        .routers
        .into_iter()
        .find(|r| r.id == router_id)?;
    while serde_json::to_vec(&r).map_or(usize::MAX, |b| b.len()) > BUDGET {
        let Some(lists) = r.lists.as_mut() else { break };
        if lists.dhcp_clients.pop().is_none() {
            break;
        }
        lists.truncated = true;
    }
    Some(r)
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

/// A connection test through the service API socket (`router.test`). The service owns router I/O;
/// the broker has no network access. A missing or slow service gives the class `service`.
pub async fn core_test(state_dir: &Path, actor: &str, router_id: &str) -> RouterTestResult {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
    let fallback = RouterTestResult {
        router_id: router_id.into(),
        class: "service".into(),
        at: crate::broker::now(),
        ..Default::default()
    };
    let exchange = async {
        let stream = tokio::net::UnixStream::connect(state_dir.join("api.sock")).await?;
        let (read, mut write) = stream.into_split();
        let mut line = serde_json::to_vec(&serde_json::json!({
            "v": 1, "id": "web-test", "op": "router.test",
            "args": {"router_id": router_id, "actor": actor},
        }))?;
        line.push(b'\n');
        write.write_all(&line).await?;
        let mut answer = String::new();
        tokio::io::BufReader::new(read)
            .take(64 * 1024)
            .read_line(&mut answer)
            .await?;
        let value: serde_json::Value = serde_json::from_str(&answer)?;
        anyhow::ensure!(value["ok"] == true, "the service refused the test");
        Ok::<_, anyhow::Error>(serde_json::from_value::<RouterTestResult>(
            value["result"].clone(),
        )?)
    };
    match tokio::time::timeout(std::time::Duration::from_secs(60), exchange).await {
        Ok(Ok(result)) if result.router_id == router_id => result,
        _ => fallback,
    }
}

/// `lqosync3 sync-now`: the same command binary, without a request body.
pub async fn run_sync(editor: &crate::broker::Editor) -> RouterEditOutcome {
    command(editor, "sync-now", None).await
}

/// Run the core command once with a time limit. Its answer is the outcome; anything else is internal.
pub async fn run_editor(
    editor: &crate::broker::Editor,
    request: &RouterEditRequest,
) -> RouterEditOutcome {
    command(
        editor,
        "router-edit",
        Some(serde_json::to_vec(request).unwrap_or_default()),
    )
    .await
}

async fn command(
    editor: &crate::broker::Editor,
    name: &str,
    body: Option<Vec<u8>>,
) -> RouterEditOutcome {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fail = |m: &str| RouterEditOutcome {
        code: "internal".into(),
        message: m.into(),
        reply: None,
    };
    let Ok(mut child) = tokio::process::Command::new(&editor.bin)
        .arg(name)
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
    let run = async {
        stdin.write_all(&body.unwrap_or_default()).await?;
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
