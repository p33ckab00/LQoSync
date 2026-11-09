# Cycle collection and subscriber state implementation plan

Owner choice: refresh only with the existing sync cycle. Do not add router polling or shorten the shaping interval.

- [x] Add a sanitized collection cache from the same successful RouterOS reads. Include enabled sources, raw counts, PPP inventory/session state, Hotspot active sessions, and DHCP lease state. Preserve failed-router evidence as stale; never turn a failed read into offline users.
- [x] Publish atomically from Runner without changing shaping assembly, grace, removals, or reload behavior. Add regression tests for disabled sources, failures, duplicate identities, DHCP bound/unbound, and secret omission. Read local Hotspot account names/disabled flags in the same cycle when that source is enabled.
- [x] Load the bounded cache outside draw. Project observed subscribers separately from shaping inputs, including inactive PPP accounts. Keep configured shaping preview unchanged.
- [x] Wire Dashboard, Subscribers, and Routers to real source counts/state. Show timestamps, stale/unknown/disabled states, and DHCP lease semantics. Test filters, independent data, and freshness expiry.
- [x] Review performance/integrity, run full checks, update docs/captures/brain, and prepare a verified release under the existing deployment workflow.

Acceptance: actual table-path counts work; current cycle states reach rows/details/filters; no background router polling; stale data cannot claim a live state; no credentials enter the public cache.
