# lqosync3 Telegram Implementation Plan (Plan 2b)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Send Telegram messages for changes, errors, router states, and guard holds, from the `Status` of each cycle.

**Architecture:** `src/notify.rs` has a pure part (event memory, message text, split) and a transport part (HTTP/1.1 POST over any async stream, TLS over TCP). `main.rs` calls the notifier after each cycle. A send failure is a log line only.

**Tech Stack:** Rust, tokio, rustls 0.23 (ring), tokio-rustls 0.26, webpki-roots, serde_json.

**Spec:** `docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md`

## Global Constraints

- All defaults in `config/defaults.toml`. No default value in code.
- The token never appears in a log line, an error, or `status.json`.
- A Telegram failure never changes the cycle result or the files.
- Docs and comments in ASD-STE100.

## Review Focus

1. A send that hangs (no answer from the server): the cycle loop must continue after `timeout`.
2. An error text that changes each cycle (for example a time in the text): one message for each change is the expected behavior.
3. A circuit name with `<` or `&`: the HTML must stay valid.
4. A message with one line longer than `max_message_chars`: the split must cut it and must not loop.
5. `bot_token_env` set to a variable that is empty: use the file.

---

### Task 1: Config keys

**Files:** `config/defaults.toml`, `src/config.rs`, `docs/CONFIGURATION.md`

- [ ] Test `telegram_needs_a_chat_id_when_enabled`: `enabled = true` with empty `chat_id` and `chat_id_env` is an error. `max_message_chars = 50` is an error.
- [ ] Add `api_host`, `api_port`. Remove `include_computation`. Add the validation.

### Task 2: Events and message (`src/notify.rs`)

**Produces:** `pub struct Notifier` (memory), `pub fn Notifier::message(&mut self, tg: &Telegram, st: &Status) -> Option<String>`, `pub fn split(text: &str, max: usize) -> Vec<String>`, `fn esc(&str) -> String`.

- [ ] Tests: no event gives `None`; files changed gives a message with the added names; shadow title starts with `[shadow]`; the same error two times gives one message; error then no error gives "cleared"; router online to failed gives an alert, and back to online gives a message; first cycle with an online router and no change gives `None`; `notify_on_changes = false` skips a change; list truncation `...and N more`; `<b>` in a name is escaped; disabled gives `None`.
- [ ] Tests for `split`: parts at line ends, each part `<= max` chars, a long line is cut.

### Task 3: Transport

**Produces:** `pub async fn post_json<S: AsyncRead + AsyncWrite + Unpin>(stream: S, host: &str, path: &str, body: &str) -> Result<(), String>`, `pub async fn send(tg: &Telegram, text: &str, owner_uid: u32) -> Result<(), String>`, `pub fn token(tg: &Telegram, owner_uid: u32) -> Result<String, String>`, `pub fn chat_id(tg: &Telegram) -> String`.

- [ ] Test `post_json_sends_request_and_reads_status` with a local `TcpListener`: request line `POST /botT/sendMessage HTTP/1.1`, `Host`, `Content-Type: application/json`, `Content-Length`, body. Answer `200` gives `Ok`. Answer `400` with `{"description":"Bad Request: chat not found"}` gives an error with the description and without the path.
- [ ] Test `token_env_wins_and_empty_env_uses_file`.
- [ ] Test `send_times_out` with a server that never answers.

### Task 4: Wire into `run`, `once`, and `check`

**Files:** `src/main.rs`, `src/cli.rs`, docs.

- [ ] Test in `cli.rs`: `check` with `enabled = true` reports a missing token file as an error, and no "Plan 2b" warning.
- [ ] `main.rs`: one `Notifier` in `run`; after each cycle, if `message()` gives text, send each part. Log a failure as `warning: telegram: <error>`.
- [ ] Docs: CONFIGURATION.md (Telegram section), OPERATIONS.md (token install), HANDOFF.
