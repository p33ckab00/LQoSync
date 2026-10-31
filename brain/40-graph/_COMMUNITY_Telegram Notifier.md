---
type: community
members: 75
---

# Telegram Notifier

**Members:** 75 nodes

## Members
- [[dot-from_report()]] - code - src/status.rs
- [[dot-message()]] - code - src/notify.rs
- [[dot-to_json()]] - code - src/status.rs
- [[ClientConfig]] - code
- [[Event memory (no repeat, no retry, in-process)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Global constraints (no token leak, failure never changes cycle)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[HTML message format and split]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[HTTPS delivery with webpki-roots TLS]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[JoinHandle]] - code
- [[Notifier]] - code - src/notify.rs
- [[Out of scope (start message, retry, other channels)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Plan 2b deferred minor items]] - concept - brain/HANDOFF.md
- [[Plan 2b lqosync3 Telegram implementation]] - document - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Report_1]] - code
- [[ReportSummary]] - code - src/status.rs
- [[Review focus (hanging send, changing error text, HTML escape, long line split, empty env)]] - rationale - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[RouterStatus]] - code - src/status.rs
- [[S]] - code
- [[Secret file rule (0600, root, _file_env)]] - rationale - docs/CONFIGURATION.md
- [[Status]] - code - src/status.rs
- [[Task 1 Config keys]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Task 2 Events and message]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Task 3 Transport (post_json, send, token, chat_id)]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Task 4 Wire into run, once, check]] - concept - docs/superpowers/plans/2026-10-31-lqosync3-telegram.md
- [[Telegram]] - code - src/config.rs
- [[Telegram config changes (api_host, api_port, drop include_computation)]] - concept - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Telegram notifier design (Plan 2b spec)]] - document - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Telegram settings]] - concept - docs/CONFIGURATION.md
- [[Token and chat ID sources (env first, then file)]] - rationale - docs/superpowers/specs/2026-10-31-lqosync3-telegram-design.md
- [[Turn on Telegram procedure]] - concept - docs/OPERATIONS.md
- [[a_config_error_is_an_error_event()]] - code - src/notify.rs
- [[a_long_multi_line_error_splits_into_valid_html_parts()]] - code - src/notify.rs
- [[a_very_long_error_is_cut()]] - code - src/notify.rs
- [[an_empty_token_is_an_error()]] - code - src/notify.rs
- [[an_error_is_sent_once_then_cleared()]] - code - src/notify.rs
- [[asyncread]] - concept
- [[asyncreadext]] - concept
- [[asyncwrite]] - concept
- [[asyncwriteext]] - concept
- [[body_complete()]] - code - src/notify.rs
- [[btreemap]] - concept
- [[cap()]] - code - src/notify.rs
- [[chat_id()]] - code - src/notify.rs
- [[code_block()]] - code - src/notify.rs
- [[disabled_gives_no_message()]] - code - src/notify.rs
- [[esc()]] - code - src/notify.rs
- [[files_changed_lists_the_circuits()]] - code - src/notify.rs
- [[first_cycle_sends_only_a_router_that_is_not_online()]] - code - src/notify.rs
- [[guard_alerts_are_sent_when_they_change()]] - code - src/notify.rs
- [[list()]] - code - src/notify.rs
- [[lists_are_cut_and_names_are_escaped()]] - code - src/notify.rs
- [[no_event_gives_no_message()]] - code - src/notify.rs
- [[notify.rs]] - code - src/notify.rs
- [[notify_on_changes_off_skips_a_change()]] - code - src/notify.rs
- [[oncelock]] - concept
- [[post_json()]] - code - src/notify.rs
- [[post_json_error_has_the_description_and_not_the_path()]] - code - src/notify.rs
- [[post_json_sends_request_and_reads_status()]] - code - src/notify.rs
- [[post_json_stops_at_content_length_without_a_close()]] - code - src/notify.rs
- [[router_state_changes_are_sent()]] - code - src/notify.rs
- [[send()]] - code - src/notify.rs
- [[send_all()]] - code - src/notify.rs
- [[send_times_out_on_a_silent_server()]] - code - src/notify.rs
- [[server()]] - code - src/notify.rs
- [[shadow_title_has_the_prefix()]] - code - src/notify.rs
- [[split()]] - code - src/notify.rs
- [[split_cuts_at_line_ends()]] - code - src/notify.rs
- [[st()]] - code - src/notify.rs
- [[status.rs]] - code - src/status.rs
- [[status_json_has_the_spec_fields()]] - code - src/status.rs
- [[super]] - concept
- [[tg()]] - code - src/notify.rs
- [[tls_config()]] - code - src/notify.rs
- [[token()]] - code - src/notify.rs
- [[token_env_wins_and_empty_env_uses_file()]] - code - src/notify.rs

## Live Query (requires Dataview plugin)

```dataview
TABLE source_file, type FROM #community/Telegram_Notifier
SORT file.name ASC
```

## Connections to other communities
- 7 edges to [[_COMMUNITY_Router State]]
- 6 edges to [[_COMMUNITY_MikroTik Session]]
- 5 edges to [[_COMMUNITY_MikroTik Session 2]]
- 4 edges to [[_COMMUNITY_Speed Rules]]
- 3 edges to [[_COMMUNITY_Telegram Notifier 2]]
- 3 edges to [[_COMMUNITY_Session Notes]]
- 3 edges to [[_COMMUNITY_Telegram Notifier 3]]
- 2 edges to [[_COMMUNITY_Config]]
- 2 edges to [[_COMMUNITY_deserialize]]
- 2 edges to [[_COMMUNITY_Status and Diff]]
- 2 edges to [[_COMMUNITY_Config 2]]
- 2 edges to [[_COMMUNITY_Topology]]
- 2 edges to [[_COMMUNITY_Circuit Collection]]
- 2 edges to [[_COMMUNITY_CLI and Checks]]
- 2 edges to [[_COMMUNITY_Status and Diff 2]]
- 1 edge to [[_COMMUNITY_Config 3]]
- 1 edge to [[_COMMUNITY_CertificateDer]]
- 1 edge to [[_COMMUNITY_hashmap]]
- 1 edge to [[_COMMUNITY_LibreQoS v3]]
- 1 edge to [[_COMMUNITY_Router State 2]]
- 1 edge to [[_COMMUNITY_Secrets]]
- 1 edge to [[_COMMUNITY_File Safety]]
- 1 edge to [[_COMMUNITY_fs]]
- 1 edge to [[_COMMUNITY_Router State 3]]

## Top bridge nodes
- [[super]] - degree 17, connects to 14 communities
- [[btreemap]] - degree 10, connects to 8 communities
- [[notify.rs]] - degree 52, connects to 4 communities
- [[Task 4 Wire into run, once, check]] - degree 4, connects to 3 communities
- [[Telegram notifier design (Plan 2b spec)]] - degree 10, connects to 2 communities