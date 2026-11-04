# Controlled-language requirements

`SHALL` means required. IDs are stable for design reviews and tests.

- CORE-01: LQoSync4 SHALL read subscriber data from configured MikroTik routers.
- CORE-02: LQoSync4 SHALL validate its circuit model before it writes LibreQoS input.
- CORE-03: Shadow mode SHALL NOT change LibreQoS files or MikroTik configuration.
- CORE-04: The core port SHALL preserve validated LQoSync3 output formats.
- CORE-05: LQoSync4 SHALL NOT include the LQoSync3 TUI.
- UI-01: The web UI SHALL show a left sidebar and a top bar.
- UI-02: The dashboard SHALL show subscriber, router, upstream, traffic, and LibreQoS status.
- UI-03: Each status view SHALL show its last successful update time.
- UI-04: An operator SHALL be able to find a subscriber and view its upstream.
- UI-05: The load balancer SHALL show Allocations, Assignments, Discovery, and History.
- WAN-01: The engine SHALL support percentage, subscriber-count, and manual policies.
- WAN-02: The engine SHALL keep a valid subscriber assignment stable.
- WAN-03: The engine SHALL NOT assign a subscriber to a known-down upstream.
- WAN-04: The system SHALL show a diff before it applies RouterOS changes.
- WAN-05: The system SHALL record the cause and result of each assignment change.
- WAN-06: MikroTik core SHALL own packet routing and session behavior after assignment changes.
- PROF-01: A profile SHALL have baseline download and upload limits.
- PROF-02: A schedule override SHALL have days, a time window, limits, and priority.
- PROF-03: The UI SHALL show effective profile values and their source.
- SAFE-01: The API SHALL authorize each state-changing action.
- SAFE-02: The system SHALL NOT expose credentials in API responses or audit records.
- IP-01: Future telemetry SHALL associate destination IP observations with subscriber and time.
- IP-02: IP enrichment SHALL show ASN and organization when data is available.
- IP-03: CDN and app labels SHALL show confidence or unknown status.
