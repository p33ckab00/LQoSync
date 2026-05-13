# Policy Center Settings Guidelines

This guide is the operator-facing explanation for every visible Policy Center setting. It is intended as setup guidance, not just developer documentation.

## Cleanup action meanings

- **preserve_rows** — Keep existing rows and do not delete stale entries. Safest when the source may be temporarily unavailable.
- **warn_only** — Show a warning but do not remove rows. Useful while tuning policies.
- **cleanup_immediate** — Remove stale rows in the same sync cycle. Fastest, but can cause more LibreQoS applies if clients flap.
- **cleanup_next_run** — Mark stale rows and remove them on the next successful run. Safer than immediate cleanup.
- **require_confirm_immediate** — Ask operator confirmation first, then allow same-cycle cleanup after confirmation.
- **require_confirm_next_run** — Ask operator confirmation first, then apply cleanup on the next successful run. Recommended for risky changes.
- **block_cleanup** — Prevent cleanup. Existing rows are preserved until the issue is fixed or policy is changed.
- **block_apply** — Block LibreQoS apply for this condition. Used for dangerous validation failures.

## Policy settings by section

### Preset

#### Preset mode

- **Config path:** `policies.mode`
- **Type:** `select`
- **Allowed values:** `conservative`, `balanced`, `aggressive`, `custom`
- **Recommended:** `balanced`
- **Risk:** `low`
- **What it controls:** Selects the active policy preset. Conservative is strict, Balanced is recommended for production, Aggressive prioritizes speed, and Custom means the operator manually changed individual settings.
- **Setup guide:** Start with Balanced. Use Conservative for live networks where accidental deletion is unacceptable. Use Aggressive only for lab/highly dynamic environments. Any manual policy edit should save as Custom.
- **Risk note:** Changing presets can modify many cleanup/apply rules at once. Run Dry Run after applying a preset.

### Cleanup Core

#### Cleanup policy engine

- **Config path:** `policies.cleanup.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Turns the Smart Cleanup Policy Engine on or off. When enabled, LQoSync classifies why rows are stale before deciding whether to delete, preserve, confirm, or block.
- **Setup guide:** Keep enabled. Disabling this returns cleanup behavior closer to simple sync logic and removes important protection.
- **Risk note:** Disabling cleanup intelligence can allow unintended stale-row removal depending on older code paths.

#### Global default cleanup action

- **Config path:** `policies.cleanup.global_default_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Fallback cleanup action used when no source-specific or reason-specific policy matches a cleanup candidate.
- **Setup guide:** Use require_confirm_next_run for conservative production behavior. Use cleanup_next_run for a faster but still staged workflow.
- **Risk note:** Avoid cleanup_immediate as the global default unless the operator accepts fast deletion for all sources.

#### Confirmation expiry hours

- **Config path:** `policies.cleanup.confirmation_expires_hours`
- **Type:** `number`
- **Recommended:** `24`
- **Risk:** `medium`
- **What it controls:** Controls how long a pending cleanup confirmation remains valid before the operator must confirm again.
- **Setup guide:** 24 hours is a good default. Use shorter values if many operators change config; use longer values for planned migrations.
- **Risk note:** Very long expiry can apply an old confirmation after the network/config has changed.

#### Confirmed cleanup apply mode

- **Config path:** `policies.cleanup.apply_confirmed_cleanup`
- **Type:** `select`
- **Allowed values:** `immediate`, `next_run`
- **Recommended:** `next_run`
- **Risk:** `medium`
- **What it controls:** Controls when cleanup happens after the operator confirms a pending cleanup decision.
- **Setup guide:** Use next_run for production so LQoSync re-checks current config and source state before deleting. Use immediate for urgent manual cleanup.
- **Risk note:** Immediate confirmed cleanup can remove rows before another full collection confirms the condition.

#### Allow immediate cleanup

- **Config path:** `policies.cleanup.allow_immediate_cleanup`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Master permission that allows any policy to delete stale rows in the same sync cycle.
- **Setup guide:** Enable if DHCP/Hotspot should update quickly. Disable if all deletions must be staged or confirmed first.
- **Risk note:** If enabled with aggressive source policies, dynamic clients can cause more file churn and LibreQoS applies.

### PPPoE Cleanup

#### PPPoE cleanup policy

- **Config path:** `policies.cleanup_sources.pppoe.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables the source-specific cleanup policy block for PPPoE. When disabled, global cleanup defaults are used for this source.
- **Setup guide:** Keep enabled if PPPoE should have its own behavior for inactive, disabled, failed, zero-result, and mass-removal cases.
- **Risk note:** Disabling source-specific policies can make the source follow broader defaults that may be too aggressive or too conservative.

#### Normal inactive action

- **Config path:** `policies.cleanup_sources.pppoe.normal_inactive_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `cleanup_next_run`
- **Risk:** `high`
- **What it controls:** Action when a PPPoE account that was previously active is no longer active during a normal scan.
- **Setup guide:** cleanup_next_run is recommended because PPPoE usernames are stable but sessions can reconnect shortly.
- **Risk note:** cleanup_immediate can remove/add the same subscriber if PPP reconnects quickly.

#### Source disabled action

- **Config path:** `policies.cleanup_sources.pppoe.source_disabled_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when PPPoE collection is disabled in config and existing PPPoE rows would disappear.
- **Setup guide:** Use require_confirm_next_run because this is an intentional but high-impact operator change.
- **Risk note:** cleanup_immediate can remove all PPPoE rows if the source is disabled by mistake.

#### Collector failed action

- **Config path:** `policies.cleanup_sources.pppoe.collector_failed_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `preserve_rows`
- **Risk:** `critical`
- **What it controls:** Action when PPPoE is enabled but MikroTik API collection fails.
- **Setup guide:** Use preserve_rows. API failure is not proof that subscribers are gone.
- **Risk note:** Deleting on collector failure can wipe valid PPPoE clients from LibreQoS.

#### Zero-result action

- **Config path:** `policies.cleanup_sources.pppoe.zero_result_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `block_cleanup`
- **Risk:** `critical`
- **What it controls:** Action when PPPoE collection succeeds but returns zero rows while enabled.
- **Setup guide:** Use block_cleanup or require_confirm_next_run unless zero active PPP users is normal for your network.
- **Risk note:** Zero result after previous success may indicate API/profile/query issues.

#### Mass-removal action

- **Config path:** `policies.cleanup_sources.pppoe.mass_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when PPPoE removal exceeds node/source guard thresholds.
- **Setup guide:** Use require_confirm_next_run so the operator reviews the impact.
- **Risk note:** Immediate mass PPPoE cleanup can remove many active subscribers if detection is wrong.

#### Respect percentage/count guards

- **Config path:** `policies.cleanup_sources.pppoe.respect_percentage_guards`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Allows node/source percentage and count guards to override normal PPPoE cleanup behavior.
- **Setup guide:** Keep enabled for PPPoE because PPP usernames represent real subscribers.
- **Risk note:** Turning off guards makes PPPoE cleanup more aggressive.

### DHCP Cleanup

#### DHCP cleanup policy

- **Config path:** `policies.cleanup_sources.dhcp.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables the source-specific cleanup policy block for DHCP. When disabled, global cleanup defaults are used for this source.
- **Setup guide:** Keep enabled if DHCP should have its own behavior for inactive, disabled, failed, zero-result, and mass-removal cases.
- **Risk note:** Disabling source-specific policies can make the source follow broader defaults that may be too aggressive or too conservative.

#### Normal inactive action

- **Config path:** `policies.cleanup_sources.dhcp.normal_inactive_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `cleanup_immediate`
- **Risk:** `high`
- **What it controls:** Action when a DHCP lease/client disappears during normal operation.
- **Setup guide:** Use cleanup_immediate for dynamic/PisoWiFi-style DHCP, or cleanup_next_run for subscriber DHCP.
- **Risk note:** Immediate cleanup is fast but can increase LibreQoS apply frequency if leases flap.

#### Source disabled action

- **Config path:** `policies.cleanup_sources.dhcp.source_disabled_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when DHCP collection or a DHCP server source is disabled and existing DHCP rows would disappear.
- **Setup guide:** Use require_confirm_next_run because disabling a source can remove many rows intentionally.
- **Risk note:** Immediate cleanup can remove rows because of a config mistake.

#### Collector failed action

- **Config path:** `policies.cleanup_sources.dhcp.collector_failed_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `preserve_rows`
- **Risk:** `critical`
- **What it controls:** Action when DHCP is enabled but lease collection fails.
- **Setup guide:** Use preserve_rows. Failure to read leases is not proof that clients are gone.
- **Risk note:** Deleting rows on failed collection can remove valid clients.

#### Zero-result action

- **Config path:** `policies.cleanup_sources.dhcp.zero_result_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `block_cleanup`
- **Risk:** `critical`
- **What it controls:** Action when DHCP scan succeeds but returns zero leases while DHCP is enabled.
- **Setup guide:** Use block_cleanup by default. A zero result may mean VLAN/API/DHCP source issue.
- **Risk note:** cleanup_immediate can wipe DHCP rows if the scan result is wrong.

#### Mass-removal action

- **Config path:** `policies.cleanup_sources.dhcp.mass_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when DHCP removal exceeds source/node guard thresholds.
- **Setup guide:** require_confirm_next_run is safest. If DHCP is intentionally dynamic, adjust respect_percentage_guards.
- **Risk note:** Mass DHCP cleanup can be normal in guest networks but dangerous in subscriber networks.

#### Respect percentage/count guards

- **Config path:** `policies.cleanup_sources.dhcp.respect_percentage_guards`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `medium`
- **What it controls:** Controls whether mass-removal guards can override DHCP normal cleanup.
- **Setup guide:** Disable for highly dynamic DHCP; enable for subscriber DHCP.
- **Risk note:** Disabling guards makes DHCP cleanup faster but less protected.

### Hotspot Cleanup

#### Hotspot cleanup policy

- **Config path:** `policies.cleanup_sources.hotspot.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables the source-specific cleanup policy block for Hotspot. When disabled, global cleanup defaults are used for this source.
- **Setup guide:** Keep enabled if Hotspot should have its own behavior for inactive, disabled, failed, zero-result, and mass-removal cases.
- **Risk note:** Disabling source-specific policies can make the source follow broader defaults that may be too aggressive or too conservative.

#### Normal inactive action

- **Config path:** `policies.cleanup_sources.hotspot.normal_inactive_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `cleanup_immediate`
- **Risk:** `high`
- **What it controls:** Action when Hotspot active users/sessions disappear normally.
- **Setup guide:** cleanup_immediate is usually acceptable for session-style Hotspot. Use cleanup_next_run if users flap often.
- **Risk note:** Immediate cleanup may cause more applies in busy captive/session environments.

#### Source disabled action

- **Config path:** `policies.cleanup_sources.hotspot.source_disabled_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when Hotspot collection is disabled and existing Hotspot rows would disappear.
- **Setup guide:** cleanup_next_run or require_confirm_next_run are safer than immediate deletion.
- **Risk note:** Immediate deletion can remove all Hotspot rows if disabled accidentally.

#### Collector failed action

- **Config path:** `policies.cleanup_sources.hotspot.collector_failed_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `preserve_rows`
- **Risk:** `critical`
- **What it controls:** Action when Hotspot is enabled but active-user collection fails.
- **Setup guide:** Use preserve_rows because a read failure is not proof users are gone.
- **Risk note:** Deleting on failure can remove valid active sessions.

#### Zero-result action

- **Config path:** `policies.cleanup_sources.hotspot.zero_result_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `warn_only`
- **Risk:** `critical`
- **What it controls:** Action when Hotspot scan succeeds but returns zero users.
- **Setup guide:** warn_only or cleanup_next_run can be reasonable if sessions naturally empty; block_cleanup for production sensitivity.
- **Risk note:** cleanup_immediate may be okay for small guest networks but risky after a collector anomaly.

#### Mass-removal action

- **Config path:** `policies.cleanup_sources.hotspot.mass_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when Hotspot removal exceeds thresholds.
- **Setup guide:** require_confirm_next_run is safest if Hotspot users are subscribers; warn_only/cleanup_next_run may fit guest sessions.
- **Risk note:** Mass Hotspot removal may be normal after vouchers expire but should be visible.

#### Respect percentage/count guards

- **Config path:** `policies.cleanup_sources.hotspot.respect_percentage_guards`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `medium`
- **What it controls:** Controls whether mass-removal guards can override Hotspot cleanup.
- **Setup guide:** Disable for highly dynamic sessions; enable for subscriber-like Hotspot use.
- **Risk note:** Disabling guards favors speed over safety.

### Static/manual rows Cleanup

#### Static/manual rows cleanup policy

- **Config path:** `policies.cleanup_sources.static.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables the source-specific cleanup policy block for Static/manual. When disabled, global cleanup defaults are used for this source.
- **Setup guide:** Keep enabled if Static/manual should have its own behavior for inactive, disabled, failed, zero-result, and mass-removal cases.
- **Risk note:** Disabling source-specific policies can make the source follow broader defaults that may be too aggressive or too conservative.

#### Normal inactive action

- **Config path:** `policies.cleanup_sources.static.normal_inactive_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `preserve_rows`
- **Risk:** `high`
- **What it controls:** Action when static/manual rows appear absent from generated data.
- **Setup guide:** preserve_rows is recommended because manual/static rows are operator-managed.
- **Risk note:** Automatic deletion of manual rows can remove intentionally preserved devices.

#### Source disabled action

- **Config path:** `policies.cleanup_sources.static.source_disabled_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when static/manual source behavior is disabled or excluded.
- **Setup guide:** preserve_rows unless the operator explicitly confirms removal.
- **Risk note:** Immediate cleanup can delete hand-maintained entries.

#### Collector failed action

- **Config path:** `policies.cleanup_sources.static.collector_failed_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `preserve_rows`
- **Risk:** `critical`
- **What it controls:** Action when manual/static source loading fails.
- **Setup guide:** preserve_rows. Manual rows should not disappear due to a read error.
- **Risk note:** Deleting on load failure is unsafe.

#### Zero-result action

- **Config path:** `policies.cleanup_sources.static.zero_result_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `warn_only`
- **Risk:** `critical`
- **What it controls:** Action when static/manual source returns no rows.
- **Setup guide:** preserve_rows by default.
- **Risk note:** Zero result may be a file/path/config problem.

#### Mass-removal action

- **Config path:** `policies.cleanup_sources.static.mass_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when many static/manual rows would be removed.
- **Setup guide:** preserve_rows or require_confirm_next_run.
- **Risk note:** Manual rows should not be mass-deleted automatically.

#### Respect percentage/count guards

- **Config path:** `policies.cleanup_sources.static.respect_percentage_guards`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Allows mass-removal guards to protect manual/static rows.
- **Setup guide:** Keep enabled.
- **Risk note:** Disabling can allow aggressive cleanup of manual data.

### Mass Removal Guards

#### Node removal guard

- **Config path:** `policies.node_cleanup_guard.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Enables protection for individual generated nodes such as a DHCP server node, PPP plan node, or Hotspot node.
- **Setup guide:** Keep enabled so one node losing many clients is detected before cleanup/apply.
- **Risk note:** Disabling can allow a broken source/node to delete many rows.

#### Node removal threshold percent

- **Config path:** `policies.node_cleanup_guard.threshold_percent`
- **Type:** `number`
- **Recommended:** `30`
- **Risk:** `high`
- **What it controls:** Percentage of clients removed from one node before the node guard can trigger.
- **Setup guide:** 30% is a good default. Lower is stricter; higher is more permissive.
- **Risk note:** Percentage alone is not enough for small nodes; min_node_size and min_removed_count also apply.

#### Minimum node size

- **Config path:** `policies.node_cleanup_guard.min_node_size`
- **Type:** `number`
- **Recommended:** `10`
- **Risk:** `medium`
- **What it controls:** Minimum previous node size required before percentage-based node protection applies.
- **Setup guide:** Use 10 so a small node with 3 clients does not block just because 1 client disappeared.
- **Risk note:** Too low makes small nodes noisy; too high may miss medium-size node failures.

#### Minimum removed count

- **Config path:** `policies.node_cleanup_guard.min_removed_count`
- **Type:** `number`
- **Recommended:** `3`
- **Risk:** `medium`
- **What it controls:** Minimum number of removed rows required before percentage-based node protection applies.
- **Setup guide:** Use 3 to avoid blocking normal 1-client movement in small DHCP nodes.
- **Risk note:** Too low causes false alarms; too high can miss real removals.

#### Node guard action

- **Config path:** `policies.node_cleanup_guard.action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action taken when one generated node exceeds node removal thresholds.
- **Setup guide:** require_confirm_next_run is safest. cleanup_next_run is faster. block_cleanup is strictest.
- **Risk note:** cleanup_immediate here can delete many rows from one node without review.

#### Small-node guard

- **Config path:** `policies.small_node_guard.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Uses special behavior for small nodes so raw percentages do not overreact to one client disappearing.
- **Setup guide:** Keep enabled. It prevents cases like 1 of 3 clients removed from being treated as a dangerous 33% mass removal.
- **Risk note:** Disabling means percentage thresholds may be noisy on tiny nodes.

#### Small-node max size

- **Config path:** `policies.small_node_guard.max_node_size`
- **Type:** `number`
- **Recommended:** `5`
- **Risk:** `medium`
- **What it controls:** Defines what counts as a small node for small-node handling.
- **Setup guide:** 5 is a practical default for small DHCP/Hotspot groups.
- **Risk note:** Higher values make more nodes bypass normal percentage logic.

#### Small-node partial removal

- **Config path:** `policies.small_node_guard.partial_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `cleanup_next_run`
- **Risk:** `medium`
- **What it controls:** Action when only some clients disappear from a small node.
- **Setup guide:** cleanup_next_run is a balanced default. cleanup_immediate is acceptable for dynamic DHCP/Hotspot if operator wants fast cleanup.
- **Risk note:** require_confirm for every small-node partial removal can create too many prompts.

#### Small-node full removal

- **Config path:** `policies.small_node_guard.full_removal_action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action when all clients disappear from a small node.
- **Setup guide:** require_confirm_next_run is recommended because 100% removal, even on a small node, may indicate source/config trouble.
- **Risk note:** cleanup_immediate can delete all rows from a small node without review.

#### Source removal guard

- **Config path:** `policies.source_cleanup_guard.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Protects an entire source, such as all PPPoE, all DHCP, or all Hotspot rows, from large unexpected removal.
- **Setup guide:** Keep enabled in production. Source-wide drops are usually high-risk unless intentionally disabled.
- **Risk note:** Disabling removes protection against source-wide API/config mistakes.

#### Source threshold percent

- **Config path:** `policies.source_cleanup_guard.threshold_percent`
- **Type:** `number`
- **Recommended:** `30`
- **Risk:** `high`
- **What it controls:** Percentage of a whole source that must disappear before the source guard triggers.
- **Setup guide:** 30% is a good production default. Adjust higher if the source is naturally volatile.
- **Risk note:** A threshold too high may allow accidental mass cleanup.

#### Source minimum removed count

- **Config path:** `policies.source_cleanup_guard.min_removed_count`
- **Type:** `number`
- **Recommended:** `5`
- **Risk:** `medium`
- **What it controls:** Minimum removed rows required before source percentage protection applies.
- **Setup guide:** 5 prevents small source groups from constantly requiring confirmation.
- **Risk note:** Too high may ignore meaningful losses in small deployments.

#### Source guard action

- **Config path:** `policies.source_cleanup_guard.action`
- **Type:** `select`
- **Allowed values:** `preserve_rows`, `warn_only`, `cleanup_immediate`, `cleanup_next_run`, `require_confirm_immediate`, `require_confirm_next_run`, `block_cleanup`, `block_apply`
- **Recommended:** `require_confirm_next_run`
- **Risk:** `high`
- **What it controls:** Action taken when source-wide mass-removal threshold is exceeded.
- **Setup guide:** require_confirm_next_run is recommended. block_cleanup is stricter. cleanup_immediate is not recommended for production.
- **Risk note:** This can override source-specific immediate cleanup if respect_percentage_guards is enabled.

### Apply Guards

#### Block apply on collector failure

- **Config path:** `policies.apply_guard.block_apply_on_collector_failure`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Prevents LibreQoS apply when a source collector failed and output may be incomplete.
- **Setup guide:** Keep enabled in production.
- **Risk note:** Applying after collector failure can remove valid clients from shaping.

#### Block apply on missing parent

- **Config path:** `policies.apply_guard.block_apply_on_missing_parent`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks apply when ShapedDevices rows reference Parent Nodes missing from network.json.
- **Setup guide:** Keep enabled. Fix topology or parent naming before applying.
- **Risk note:** Missing parents can break expected hierarchy/shaping placement.

#### Block apply on duplicate IP

- **Config path:** `policies.apply_guard.block_apply_on_duplicate_ip`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks apply when duplicate IPv4 values are detected in generated rows.
- **Setup guide:** Keep enabled unless duplicates are intentionally handled elsewhere.
- **Risk note:** Duplicate IPs can cause wrong shaping assignment.

#### Block apply on invalid speed

- **Config path:** `policies.apply_guard.block_apply_on_invalid_speed`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks apply when speed values cannot be parsed or are invalid.
- **Setup guide:** Keep enabled. Fix plan comments/profile names/default speeds.
- **Risk note:** Invalid speeds can create bad or failed LibreQoS config.

#### Require manual confirm on medium risk

- **Config path:** `policies.apply_guard.require_manual_confirm_on_medium_risk`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Requires operator review for medium-risk policy outcomes.
- **Setup guide:** Keep enabled for production. Disable only if you want more automation.
- **Risk note:** Disabling lets medium-risk changes auto-apply if other settings allow it.

#### Allow auto-apply on low risk

- **Config path:** `policies.apply_guard.allow_auto_apply_on_low_risk`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Allows low-risk changes to run LibreQoS automatically.
- **Setup guide:** Enable for efficient normal operations.
- **Risk note:** Disable if you want every apply to be manual.

### Collector Guards

#### Block cleanup if source failed

- **Config path:** `policies.collector_guard.block_cleanup_if_source_failed`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Stops cleanup for a source when its collection failed.
- **Setup guide:** Keep enabled. Preserve rows until a successful scan confirms state.
- **Risk note:** Disabling can delete clients because of temporary API failure.

#### Block cleanup if enabled source returns zero

- **Config path:** `policies.collector_guard.block_cleanup_if_enabled_source_returns_zero`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Stops cleanup when an enabled source returns zero rows.
- **Setup guide:** Keep enabled unless a source naturally returns zero often.
- **Risk note:** A zero result can be a collector/router/VLAN problem.

#### Block zero-after-success cleanup

- **Config path:** `policies.collector_guard.block_cleanup_if_source_returns_zero_after_previous_success`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks cleanup when a source that previously had rows suddenly returns zero.
- **Setup guide:** Keep enabled. This catches sudden source loss.
- **Risk note:** Disabling can wipe a source after an anomaly.

#### Zero-source drop threshold percent

- **Config path:** `policies.collector_guard.zero_source_drop_threshold_percent`
- **Type:** `number`
- **Recommended:** `80`
- **Risk:** `high`
- **What it controls:** Defines the drop percentage considered suspicious when a source goes near-zero.
- **Setup guide:** 80% catches extreme drops while allowing normal changes.
- **Risk note:** Too low causes noise; too high may miss failures.

#### Warn if router API slow ms

- **Config path:** `policies.collector_guard.warn_if_router_api_slow_ms`
- **Type:** `number`
- **Recommended:** `2000`
- **Risk:** `medium`
- **What it controls:** Warns when MikroTik API collection time is slower than expected.
- **Setup guide:** 2000 ms is a practical warning threshold.
- **Risk note:** Slow API can indicate router load, network issue, or timeout risk.

### Data Quality Guards

#### Warn on fallback speed

- **Config path:** `policies.data_quality.warn_on_fallback_speed`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Warns when clients use default/fallback speed instead of comment/profile/server-derived speed.
- **Setup guide:** Keep enabled so incorrect plan detection is visible.
- **Risk note:** Fallback speeds can silently assign wrong shaping.

#### Fallback speed warning threshold

- **Config path:** `policies.data_quality.fallback_speed_warning_threshold_percent`
- **Type:** `number`
- **Recommended:** `10`
- **Risk:** `medium`
- **What it controls:** Percentage of fallback-speed clients that triggers warning.
- **Setup guide:** 10% is good for production.
- **Risk note:** Too high can hide plan-detection issues.

#### Block if fallback speed threshold

- **Config path:** `policies.data_quality.block_if_fallback_speed_threshold_percent`
- **Type:** `number`
- **Recommended:** `50`
- **Risk:** `high`
- **What it controls:** Percentage of fallback-speed clients that blocks apply.
- **Setup guide:** 50% catches severe speed-source failures.
- **Risk note:** Blocking too low can interrupt normal migration; too high may allow bad speeds.

#### Warn on missing MAC

- **Config path:** `policies.data_quality.warn_on_missing_mac`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Warns when generated rows have no MAC address.
- **Setup guide:** Keep enabled for better audit/identity quality.
- **Risk note:** Some sources may not always provide MAC; this is usually warning-only.

#### Warn on missing IP

- **Config path:** `policies.data_quality.warn_on_missing_ip`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Warns when generated rows have no IPv4 address.
- **Setup guide:** Keep enabled because LibreQoS shaping generally needs IP mapping.
- **Risk note:** Missing IP rows may not shape correctly.

### Topology Guards

#### Block missing parent nodes

- **Config path:** `policies.topology_guard.block_missing_parent_nodes`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks apply when generated Parent Node values do not exist in network.json.
- **Setup guide:** Keep enabled when using hierarchy modes.
- **Risk note:** Disabling can produce unclear or broken topology placement.

#### Block duplicate node names

- **Config path:** `policies.topology_guard.block_duplicate_node_names`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `critical`
- **What it controls:** Blocks topology/apply when duplicate node names could collide.
- **Setup guide:** Keep enabled, especially with virtual/deep hierarchy.
- **Risk note:** Duplicate names can confuse hierarchy and promotion behavior.

#### Warn on virtual node promotion

- **Config path:** `policies.topology_guard.warn_on_virtual_node_promotion`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Warns when virtual nodes may promote children to nearest physical ancestor.
- **Setup guide:** Keep enabled so operators understand LibreQoS virtual-node behavior.
- **Risk note:** Virtual nodes are useful but can surprise operators if not explained.

#### Warn on deep hierarchy depth

- **Config path:** `policies.topology_guard.warn_on_deep_hierarchy_depth`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Warns when topology depth grows beyond recommended levels.
- **Setup guide:** Keep enabled for readability and performance awareness.
- **Risk note:** Very deep trees are harder to debug.

#### Max recommended hierarchy depth

- **Config path:** `policies.topology_guard.max_recommended_depth`
- **Type:** `number`
- **Recommended:** `4`
- **Risk:** `medium`
- **What it controls:** Recommended maximum hierarchy depth before warnings appear.
- **Setup guide:** 4 is a good practical default.
- **Risk note:** Higher depth may be valid but should be deliberate.

### Backup Guards

#### Require backup before apply

- **Config path:** `policies.backup_guard.require_backup_before_apply`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Requires or expects a backup before LibreQoS apply.
- **Setup guide:** Keep enabled for production.
- **Risk note:** Applying without backups makes rollback harder.

#### Warn if backups disabled with auto-apply

- **Config path:** `policies.backup_guard.warn_if_backup_disabled_while_auto_apply_enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Warns when auto-apply is enabled but backup_before_apply is disabled.
- **Setup guide:** Keep enabled.
- **Risk note:** This exact warning is a strong production-safety signal.

#### Minimum backup retention

- **Config path:** `policies.backup_guard.minimum_backup_retention`
- **Type:** `number`
- **Recommended:** `30`
- **Risk:** `medium`
- **What it controls:** Minimum number of backups considered healthy.
- **Setup guide:** 30 gives practical rollback history.
- **Risk note:** Too low reduces rollback options.

### Anomaly Detection

#### Anomaly detection

- **Config path:** `policies.anomaly_detection.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables rule-based anomaly detection from previous successful runs.
- **Setup guide:** Keep enabled for smart warnings.
- **Risk note:** Disabling removes early warning for unusual drops/slowness.

#### Compare with last successful run

- **Config path:** `policies.anomaly_detection.compare_with_last_successful_run`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Uses last successful run as baseline for anomaly checks.
- **Setup guide:** Keep enabled.
- **Risk note:** Without baseline comparison, sudden changes are harder to classify.

#### Warn if client count drops percent

- **Config path:** `policies.anomaly_detection.warn_if_client_count_drops_percent`
- **Type:** `number`
- **Recommended:** `30`
- **Risk:** `high`
- **What it controls:** Warns when client count drops by this percentage compared with baseline.
- **Setup guide:** 30% is a practical default.
- **Risk note:** Too low can be noisy; too high may miss incidents.

#### Warn if sync duration multiplier

- **Config path:** `policies.anomaly_detection.warn_if_sync_duration_increases_multiplier`
- **Type:** `number`
- **Recommended:** `5`
- **Risk:** `medium`
- **What it controls:** Warns when sync duration is many times slower than usual.
- **Setup guide:** 5x is a practical starting point.
- **Risk note:** Slow sync may indicate API/router/system issues.

#### Warn if apply duration multiplier

- **Config path:** `policies.anomaly_detection.warn_if_apply_duration_increases_multiplier`
- **Type:** `number`
- **Recommended:** `5`
- **Risk:** `medium`
- **What it controls:** Warns when LibreQoS apply takes much longer than baseline.
- **Setup guide:** 5x is a practical starting point.
- **Risk note:** Slow apply can indicate host/load/config growth problems.

### Recommendations

#### Recommendations

- **Config path:** `policies.recommendations.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Enables operator recommendation cards.
- **Setup guide:** Keep enabled so the UI suggests the safest next action.
- **Risk note:** Disabling removes helpful guidance but not enforcement.

#### Show Why/Fix messages

- **Config path:** `policies.recommendations.show_why_fix_messages`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Shows What/Why/Fix explanations for warnings and policy decisions.
- **Setup guide:** Keep enabled for operator clarity.
- **Risk note:** Without explanations, policies can feel like hidden behavior.

#### Show operator next action

- **Config path:** `policies.recommendations.show_operator_next_action`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Shows the recommended next operator action.
- **Setup guide:** Keep enabled.
- **Risk note:** Operators may need to inspect raw logs without this guidance.

### PPPoE Stale Lifecycle

#### PPPoE identity key

- **Config path:** `policies.stale_lifecycle.sources.pppoe.identity`
- **Type:** `select`
- **Allowed values:** `username`, `server_mac`, `username_or_mac`, `manual`
- **Recommended:** `username`
- **Risk:** `medium`
- **What it controls:** Identity used to decide whether a missing client is the same client if it returns later.
- **Setup guide:** Use username for PPPoE, server_mac for DHCP, username_or_mac for Hotspot, and manual for static rows.
- **Risk note:** Grace should only be enabled when identity is stable.

#### PPPoE optional grace

- **Config path:** `policies.stale_lifecycle.sources.pppoe.grace_enabled`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `high`
- **What it controls:** Enables optional grace behavior so a missing client is held for configured runs before cleanup.
- **Setup guide:** Keep disabled by default for DHCP/Hotspot random-MAC environments. Consider enabling only for stable PPPoE usernames.
- **Risk note:** Grace can preserve ghost rows if devices change MAC/IP.

#### PPPoE grace runs

- **Config path:** `policies.stale_lifecycle.sources.pppoe.grace_runs`
- **Type:** `number`
- **Recommended:** `1`
- **Risk:** `medium`
- **What it controls:** Number of consecutive missing runs required before cleanup when grace is enabled.
- **Setup guide:** Use 1 for PPPoE if you want short reconnect tolerance; use 0 for DHCP/Hotspot unless identities are stable.
- **Risk note:** Higher values delay cleanup and may preserve stale rows.

#### PPPoE return cancels cleanup

- **Config path:** `policies.stale_lifecycle.sources.pppoe.return_cancels_cleanup`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** If the same identity returns before cleanup is applied, pending cleanup is cancelled.
- **Setup guide:** Enable for PPPoE/stable identities. Disable for unstable DHCP identities.
- **Risk note:** If identity is unstable, returns may not match the old row anyway.

### DHCP Stale Lifecycle

#### DHCP identity key

- **Config path:** `policies.stale_lifecycle.sources.dhcp.identity`
- **Type:** `select`
- **Allowed values:** `username`, `server_mac`, `username_or_mac`, `manual`
- **Recommended:** `server_mac`
- **Risk:** `medium`
- **What it controls:** Identity used to decide whether a missing client is the same client if it returns later.
- **Setup guide:** Use username for PPPoE, server_mac for DHCP, username_or_mac for Hotspot, and manual for static rows.
- **Risk note:** Grace should only be enabled when identity is stable.

#### DHCP optional grace

- **Config path:** `policies.stale_lifecycle.sources.dhcp.grace_enabled`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `high`
- **What it controls:** Enables optional grace behavior so a missing client is held for configured runs before cleanup.
- **Setup guide:** Keep disabled by default for DHCP/Hotspot random-MAC environments. Consider enabling only for stable PPPoE usernames.
- **Risk note:** Grace can preserve ghost rows if devices change MAC/IP.

#### DHCP grace runs

- **Config path:** `policies.stale_lifecycle.sources.dhcp.grace_runs`
- **Type:** `number`
- **Recommended:** `0`
- **Risk:** `medium`
- **What it controls:** Number of consecutive missing runs required before cleanup when grace is enabled.
- **Setup guide:** Use 1 for PPPoE if you want short reconnect tolerance; use 0 for DHCP/Hotspot unless identities are stable.
- **Risk note:** Higher values delay cleanup and may preserve stale rows.

#### DHCP return cancels cleanup

- **Config path:** `policies.stale_lifecycle.sources.dhcp.return_cancels_cleanup`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `low`
- **What it controls:** If the same identity returns before cleanup is applied, pending cleanup is cancelled.
- **Setup guide:** Enable for PPPoE/stable identities. Disable for unstable DHCP identities.
- **Risk note:** If identity is unstable, returns may not match the old row anyway.

### Hotspot Stale Lifecycle

#### Hotspot identity key

- **Config path:** `policies.stale_lifecycle.sources.hotspot.identity`
- **Type:** `select`
- **Allowed values:** `username`, `server_mac`, `username_or_mac`, `manual`
- **Recommended:** `username_or_mac`
- **Risk:** `medium`
- **What it controls:** Identity used to decide whether a missing client is the same client if it returns later.
- **Setup guide:** Use username for PPPoE, server_mac for DHCP, username_or_mac for Hotspot, and manual for static rows.
- **Risk note:** Grace should only be enabled when identity is stable.

#### Hotspot optional grace

- **Config path:** `policies.stale_lifecycle.sources.hotspot.grace_enabled`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `high`
- **What it controls:** Enables optional grace behavior so a missing client is held for configured runs before cleanup.
- **Setup guide:** Keep disabled by default for DHCP/Hotspot random-MAC environments. Consider enabling only for stable PPPoE usernames.
- **Risk note:** Grace can preserve ghost rows if devices change MAC/IP.

#### Hotspot grace runs

- **Config path:** `policies.stale_lifecycle.sources.hotspot.grace_runs`
- **Type:** `number`
- **Recommended:** `0`
- **Risk:** `medium`
- **What it controls:** Number of consecutive missing runs required before cleanup when grace is enabled.
- **Setup guide:** Use 1 for PPPoE if you want short reconnect tolerance; use 0 for DHCP/Hotspot unless identities are stable.
- **Risk note:** Higher values delay cleanup and may preserve stale rows.

#### Hotspot return cancels cleanup

- **Config path:** `policies.stale_lifecycle.sources.hotspot.return_cancels_cleanup`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `low`
- **What it controls:** If the same identity returns before cleanup is applied, pending cleanup is cancelled.
- **Setup guide:** Enable for PPPoE/stable identities. Disable for unstable DHCP identities.
- **Risk note:** If identity is unstable, returns may not match the old row anyway.

### Static/manual rows Stale Lifecycle

#### Static/manual rows identity key

- **Config path:** `policies.stale_lifecycle.sources.static.identity`
- **Type:** `select`
- **Allowed values:** `username`, `server_mac`, `username_or_mac`, `manual`
- **Recommended:** `manual`
- **Risk:** `medium`
- **What it controls:** Identity used to decide whether a missing client is the same client if it returns later.
- **Setup guide:** Use username for PPPoE, server_mac for DHCP, username_or_mac for Hotspot, and manual for static rows.
- **Risk note:** Grace should only be enabled when identity is stable.

#### Static/manual rows optional grace

- **Config path:** `policies.stale_lifecycle.sources.static.grace_enabled`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `high`
- **What it controls:** Enables optional grace behavior so a missing client is held for configured runs before cleanup.
- **Setup guide:** Keep disabled by default for DHCP/Hotspot random-MAC environments. Consider enabling only for stable PPPoE usernames.
- **Risk note:** Grace can preserve ghost rows if devices change MAC/IP.

#### Static/manual rows grace runs

- **Config path:** `policies.stale_lifecycle.sources.static.grace_runs`
- **Type:** `number`
- **Recommended:** `0`
- **Risk:** `medium`
- **What it controls:** Number of consecutive missing runs required before cleanup when grace is enabled.
- **Setup guide:** Use 1 for PPPoE if you want short reconnect tolerance; use 0 for DHCP/Hotspot unless identities are stable.
- **Risk note:** Higher values delay cleanup and may preserve stale rows.

#### Static/manual rows return cancels cleanup

- **Config path:** `policies.stale_lifecycle.sources.static.return_cancels_cleanup`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `low`
- **What it controls:** If the same identity returns before cleanup is applied, pending cleanup is cancelled.
- **Setup guide:** Enable for PPPoE/stable identities. Disable for unstable DHCP identities.
- **Risk note:** If identity is unstable, returns may not match the old row anyway.

### Stale Lifecycle Core

#### Stale lifecycle policy

- **Config path:** `policies.stale_lifecycle.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `medium`
- **What it controls:** Enables stale lifecycle features as a policy group.
- **Setup guide:** Keep enabled so source-aware lifecycle settings are available; per-source grace can remain disabled.
- **Risk note:** Disabling removes lifecycle visibility and grace behavior.

### Policy-Aware Auto Apply

#### Risk-aware auto apply

- **Config path:** `policies.auto_apply_policy.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `high`
- **What it controls:** Enables risk-aware auto-apply decisions using policy risk level.
- **Setup guide:** Keep enabled so low risk can apply while higher risk is held by policy.
- **Risk note:** If disabled, behavior may fall back to simpler auto_apply rules.

#### Auto apply low risk

- **Config path:** `policies.auto_apply_policy.allow_low_risk`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Allows automatic LibreQoS apply for low-risk changes.
- **Setup guide:** Enable for normal efficient operation.
- **Risk note:** Disable if all changes must be manually applied.

#### Auto apply medium risk

- **Config path:** `policies.auto_apply_policy.allow_medium_risk`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `medium`
- **What it controls:** Allows automatic LibreQoS apply for medium-risk changes.
- **Setup guide:** Keep disabled for production unless operator accepts more automation.
- **Risk note:** Medium risk may include meaningful cleanup or policy warnings.

#### Auto apply high risk

- **Config path:** `policies.auto_apply_policy.allow_high_risk`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `high`
- **What it controls:** Allows automatic LibreQoS apply for high-risk changes.
- **Setup guide:** Keep disabled.
- **Risk note:** High-risk changes should be manually reviewed.

#### Auto apply critical risk

- **Config path:** `policies.auto_apply_policy.allow_critical_risk`
- **Type:** `bool`
- **Recommended:** `False`
- **Risk:** `critical`
- **What it controls:** Allows automatic LibreQoS apply for critical-risk changes.
- **Setup guide:** Keep disabled.
- **Risk note:** Critical risk should not auto-apply in production.

#### When auto apply is held

- **Config path:** `policies.auto_apply_policy.when_blocked`
- **Type:** `select`
- **Allowed values:** `keep_pending_manual_apply`, `block_write`, `dry_run_only`
- **Recommended:** `keep_pending_manual_apply`
- **Risk:** `high`
- **What it controls:** Action when file changes exist but policy risk does not allow automatic LibreQoS apply.
- **Setup guide:** keep_pending_manual_apply is safest because files can be staged while apply waits for review.
- **Risk note:** block_write is stricter; dry_run_only is safest for testing but may prevent live updates.

### Policy Decision Trace

#### Decision trace

- **Config path:** `policies.decision_trace.enabled`
- **Type:** `bool`
- **Recommended:** `True`
- **Risk:** `low`
- **What it controls:** Stores explainable trace entries showing which policy rules influenced cleanup/write/apply decisions.
- **Setup guide:** Keep enabled for troubleshooting and support.
- **Risk note:** Turning off reduces audit clarity.

#### Max trace items

- **Config path:** `policies.decision_trace.max_items`
- **Type:** `number`
- **Recommended:** `200`
- **Risk:** `low`
- **What it controls:** Limits how many trace items are kept per policy decision.
- **Setup guide:** 200 is enough for most deployments; increase for large networks if traces are truncated.
- **Risk note:** Very high values can make state/log output larger.
