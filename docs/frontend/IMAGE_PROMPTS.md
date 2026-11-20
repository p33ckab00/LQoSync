# Image generation record

## Sign-in additions

The original sign-in used the following prompt. Two edits preserved its layout and logo mark, added superscript `v3`, and added a System theme selector.
The dark edit requested restrained frosted glass on the original dark surface.
The light edit requested the same composition with pale glass, dark text, and a blue action button.
See [THEMING.md](THEMING.md) for the image references and requested changes.

```text
Use case: ui-mockup. Generate ONE polished desktop SIGN IN SCREEN picture for LQoSync, the Rust-based ISP network configuration and orchestration web control panel. Landscape 1536x1024. Straight-on flat screen screenshot, no device mockup, no browser chrome. Match the established dark LQoSync operational design: canvas #0b0c0e, charcoal panel #17191b, thin borders #2b2e32, white #f1f3f5, secondary #b2b8bf, restrained blue #80aaff action accent. Deliberate refined industrial network-console identity per Vercel frontend-design: precise typography, confident whitespace, controlled density, no purple gradients, no decorative SaaS cards, no stock illustrations or glowing cyberpunk. IBM Plex Sans-like type with excellent readability. Composition: centered two-part login surface, restrained abstract network topology line drawing on the left taking 40% with LQoSync wordmark above it and short sentence 'Network configuration and orchestration'; quietly balanced right side with 400px-width sign-in form. Very subtle technical topology motif, no metrics or confidential operational data visible before authentication. Right form: brand small 'LQoSync', heading EXACT 'Sign in', helper 'Use your administrator account.' Visible persistent labels EXACT 'Username' and 'Password'. Username field EMPTY with understated placeholder 'Enter your username'. Password field EMPTY, placeholder 'Enter your password', eye icon and small visible 'Show' control. Clear full-width blue 'Sign in' button below. Generous 44px minimum controls; high contrast, clean alignment, no overlaps or tiny text. Small neutral assistance beneath: 'Contact your system administrator for account access.' Small footer 'LQoSync · Web control panel'. Optional top right tiny 'Design preview' label only. Inspired by Google Modern Web Guidance autofill-sign-in-form: simple visible labels, password visibility control, unambiguous submit, password-manager-friendly concept. No invented verification claims. CRITICAL: NO signup, create account, invite, forgot password, reset password, change password, social login, SSO, passkeys, role selector, superadmin count, remember me checkbox, CLI commands, or code snippets anywhere. Accounts and credentials are managed exclusively through local privileged CLI, but keep those implementation details out of the product sign-in screen. NO application sidebar before sign-in. Render a beautiful coherent single screen, not a collage and not a mobile/desktop pair. This is an image-only concept, not working HTML.
```

Date: 2026-11-20. Tool: built-in imagegen. Purpose: picture-only owner review.
The tool generated each screen independently. No reference image was used.
The prompts describe concepts, not current backend capabilities.

The written design corrects generated labels and unsupported operations.

## Shared prompt

```text
Use case: ui-mockup. Create ONE polished high fidelity desktop web application screenshot, landscape 1536x1024 or wider with crisp readable typography. This is one screen in a cohesive set of LQoSync frontend concept previews derived from LQoSync4 planning documents, for review before adapting to LQoSync3 backend. Flat straight-on UI only, fills frame; no device frame, perspective, browser chrome, photography, marketing, gradients or ornamental graphics. Consistent app brand LQoSync at top left, tiny v3 backend subtitle. Dark canvas #0b0c0e, sidebar #141517 width about 230px, panels #17191b, borders #2b2e32, text #f1f3f5, muted #b2b8bf, blue accent #80aaff. Green #2bd391 only healthy, amber #ffd077 warning, red #ff858a critical. Restrained 10px panel corners, 24px content padding, 16px gaps. IBM Plex Sans-like UI typography 14-16px body, 26px page title, tabular numbers; mono only technical identifiers. Small line icons with labels. Shared left sidebar: Dashboard, Routers; IPoE section Subscribers and Profiles; PPPoE section Users and Profiles; System section LibreQoS, Load Balancer, Users, Settings. Distinguish the two Users and Profiles destinations by section. Selected current route visibly blue/charcoal. Sidebar footer Service connected with age. Slim top bar 'Concept preview · Sample data', local time '15:30 Asia/Manila', freshness 'Updated 5s ago', account and theme control. Make clear this is conceptual sample data; no real credentials. Clear visual hierarchy, useful operational density, legible labels with no overlapping text or cropped controls. Render only the requested screen, not a collage. Include each exact page title prominently. Screen details:
```

## Screen prompts

### 01 Dashboard

```text
Five summary cards: IPoE subscribers 640, PPPoE online 324, PPPoE offline 36, routers 4, LibreQoS Imported. Large traffic area chart down/up with 1h/6h/24h controls left, WAN allocation distribution right. Lower upstream health panel with Up, Recovering, Unknown badges and allocation table listing WAN-A/B/C, capacity, members, RX, TX and utilization. Meaningful dense monitoring layout.
```

### 02 Routers

```text
Title Routers and Add router button. Filter All/Core/Access. Table 4 illustrative routers named Core-01, Access-North, Access-South, Access-East, role, endpoint, subscriber count, collection status, last read and row action. Core 10.0.0.1, other private addresses. Access-East has read error and stale timestamp, not simply offline. Right side open detail panel for Access-North with last collection details and Test connection, Request sync controls. Credential shown only as configured.
```

### 03 IPoE Subscribers

```text
IPoE section Subscribers selected. Header IPoE Subscribers with 640 discovered and filters Search subscriber, router, authorization, online state. Dense 7-row table: subscriber, MAC/IP, router, authorization, profile, online, WAN and actions. Clear DHCP data and illustrative customer names Subscriber-001 etc, private addresses. Mix Authorized, Pending, Online and Stale states. Right drawer selected Subscriber-014 showing source DHCP, observed time, current IP/MAC, profile Home 100, WAN-A, and planned Authorize / Change profile controls.
```

### 04 IPoE Profiles

```text
IPoE section Profiles selected. Header IPoE Profiles with New profile. Full-width vertical cards Home 100 baseline 100 down/30 up Mbps 320 subscribers, Home 200 200/60 240 subscribers, Business 500 500/150 80 subscribers. First profile expanded with scheduled override table Weekend boost Sat Sun 18:00-23:00 150/40 Priority 10, plus inline editor with name, day buttons, start/end, download min/max, upload min/max, priority and Add override button. Rates all explicitly labeled Mbps. Timezone Asia/Manila. Good form typography.
```

### 05 PPPoE Users

```text
PPPoE Users screen selected. Dense readable table with 7 illustrative subscriber rows. Columns Subscriber, MAC/IP, Online, Router, Profile, Speed down/up Mbps, Override, Last seen, Actions. Search/router/profile/status filters. Users named ppp-user-001 etc, private IP addresses. Inline profile select fields, occasional Scheduled or Manual speed source label. Row menu has Disable and Override concept controls. One offline row gray, one stale amber. Small selected-user detail strip below with source and age.
```

### 06 PPPoE Profiles

```text
Header PPPoE Profiles. Router filter Access-North, Sync all and New profile top right. Vertical compact full-width cards PPP-50 50/15 Mbps, PPP-100 100/30 Mbps, PPP-200 200/60 Mbps. Each card shows owning router chip, description, user count, override count, last sync with age, edit/sync/members actions. PPP-100 expanded to show scheduled evening boost table, effective-speed preview and timezone. Include small explanatory source line Router profile import versus desired policy, with Pending sync on one card.
```

### 07 LibreQoS

```text
LibreQoS screen title with Sync now control. Immediately below full-width amber Parent Node missing alert, 2 circuits / 3 rows, Apply blocked; live input unchanged, last native cycle and age, View affected circuits. Show affected circuits compact list. Two adjacent panels Last sync (candidate incomplete, previous written and imported timestamps separate) and Remote sync (Not configured). Large lower input preview panel with CSV / Network tree / Native JSON tabs, paged readable CSV 5 rows. Native target v3_network shown. Banner persists despite previous Imported state. Exact sensible status words, no false success.
```

### 08 Load Balancer Allocations

```text
Load Balancer LB1 header, Core-01 metadata and polling/data age. Five compact metric cards Qualified 1000, Allocated 960, Unallocated 40, WANs 3, Health Degraded. Tabs Allocations selected, Assignments, Discovery, History. Add allocation button. Three allocation cards WAN-A capacity 1000 Mbps target 50% members 480, WAN-B 500 Mbps target 30% members 288, WAN-C 300 Mbps target20% members192. Per-card RX/TX mini traffic chart, utilization, health labeled Up/Recovering/Unknown and pin/edit/history icon buttons with labels. Persistent small Planned routing workflow note. Footer planned changes 12 subscribers, Preview plan button.
```

### 09 Load Balancer Assignments

```text
Same LB1 Load Balancer shell as allocations; Assignments tab selected alongside Allocations, Discovery, History. Compact summary cards. Search, WAN and status filters. Full-width grouped dense assignment table with 8 sample rows: Customer, Circuit ID, MAC/IP, Online, Current WAN, Desired WAN, Pin, Sync state, Actions. Mix In sync, Pending, Pinned, Unknown. Show one row current WAN-B desired WAN-A pending. Below table plan preview panel with logical diff 12 moves and warning RouterOS writer contract pending; preview only. Show planned action controls without implying real backend availability.
```

### 10 Users

```text
Users administration screen under System selected. Header Users, Invite user button. Table of four sample local accounts operator-admin, noc-operator, noc-viewer, disabled-user; columns Role Admin/Operator/Viewer, access state Active/Disabled, last sign-in, MFA status, actions. Right side selected noc-operator detail panel with role capabilities Read monitoring, Preview changes, Request approved actions and access history link. Roles are conceptual. Small recent audit panel below lists role changed and login events without sensitive data.
```

### 11 Settings

```text
Settings screen selected. Section tabs General, Integrations, LibreQoS, Security. General selected. Main grouped form panels Deployment name LQoSync Lab, display timezone Asia/Manila, polling interval 5s. Integration status for RouterOS configured and Monitor provider Not configured. LibreQoS target Auto detected v3_network and safe sync mode selector Shadow/Apply conceptual. Security settings shows Session policy and credential references with password dots, no real secrets. Sticky bottom draft bar Unsaved changes, Discard, Review changes. Include inline validation next to a sample invalid interval field, no success claim.
```
