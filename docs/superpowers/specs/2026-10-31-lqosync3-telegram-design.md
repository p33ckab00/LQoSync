---
type: spec
created: 2026-10-31
status: accepted
related: ["[[2026-10-31-lqosync3-runtime-design]]", "[[ADR-0003-router-failed-vs-offline]]"]
---

# lqosync3 Telegram notifier (Plan 2b)

## Purpose

The Python service sends Telegram messages. The cutover to lqosync3 must not remove these messages. This spec gives the rules for the lqosync3 notifier.

## Events

The notifier reads the `status.json` data of each cycle (`Status`). It sends one message for each cycle that has one or more of these events:

| Event | Setting | Rule |
|-------|---------|------|
| Files changed | `notify_on_changes` | `files_changed` is true. In `shadow` mode, the candidate files changed. |
| Cycle error | `notify_on_error` | `error` or `config_error` is set, and the text is different from the error of the last cycle. |
| Error cleared | `notify_on_error` | An error was sent before, and the cycle has no error now. |
| Router state change | `notify_on_error` | A router state is different from the last cycle. At the first cycle, only a router that is not `online` is an event. |
| Removal guard | `notify_on_error` | `report.alerts` is not empty and is different from the last sent alerts. |

A cycle with no event sends nothing. The notifier does not repeat the same error each cycle (the Python service sends it every 30 seconds).
The memory holds the events of the last cycle, also when a send failed. A failed send is not sent again (no retry). The memory is in the process only. After a restart, the first cycle can send a router alert again.

## Message

- HTML (`parse_mode = "HTML"`). The notifier escapes `&`, `<` and `>` in all data.
- In `shadow` mode, the title starts with `[shadow]`.
- The title shows the most important event: error, router alert, guard alert, files changed, error cleared.
- Body: mode, target, files changed, reload result, next scan.
- `include_telemetry`: the cycle time, and one line for each router with its state, rows, and read time.
- Summary counts: added, updated, removed, held, duplicates, warnings.
- `include_added`, `include_updated`, `include_removed`: the circuit names. An updated circuit also shows its changed columns.
- Held circuits, duplicates, alerts, and warnings: always shown when present.
- An error text shows at most 20 lines and 1000 characters. Each line is a separate `<code>` element, so a split keeps valid HTML. The reload line shows only the first line of the reload result.
- An empty token file is an error.
- Each list shows `max_list_items` lines, then `...and N more`.
- A message longer than `max_message_chars` is split at a line end.

## Delivery

- HTTPS `POST /bot<token>/sendMessage` to `api_host:api_port` (default `api.telegram.org:443`), JSON body with `chat_id`, `text`, `parse_mode`, `disable_web_page_preview`.
- TLS with the public CA roots (`webpki-roots`). The certificate is always checked.
- `timeout` applies to each message part: TCP connect, TLS, request, and response.
- A send failure gives one log line. It never changes the cycle result, the files, or `status.json`.
- An error text never contains the token. The request path is not in any error.

## Secrets

- Token: the environment variable named in `bot_token_env` if it is set and not empty. If not, `bot_token_file` with the secret-file rules (mode without `077` bits, owner root).
- Chat ID: the environment variable named in `chat_id_env` if it is set and not empty. If not, `chat_id`.
- The notifier reads the token at each send, so a new token needs no restart.

## Config changes

- New keys: `api_host = "api.telegram.org"`, `api_port = 443`.
- Remove `include_computation`. lqosync3 has no computation report.
- `notify_on_preflight_warning` stays for LibreQoS v3 (Plan 3). It has no effect before Plan 3.
- Validation: if `enabled`, `chat_id` or `chat_id_env` must not be empty, and `max_message_chars` must be 100 to 4096.
- `lqosync3 check`: if `enabled`, check the token source and the chat ID.

## Out of scope

- A start message (the Python service also sends none).
- Retry of a failed send. The next event sends a new message.
- Other channels than Telegram.
