---
type: session
created: 2026-11-20
tags: [frontend, research, web, blueprint]
status: research
related: ["[[2026-11-18-topology-service-api]]", "[[topology-manager-status]]"]
---

# Frontend blueprint research

Checkpoint: 2026-11-20T15:26:39+08:00 (Asia/Shanghai).
Agent: Codex `/root`. Worktree: `/home/pi/LQoSync3.0`.
Branch: `lqosync3`. Source baseline: `9a465e7`.

## Owner request

Research the LQoSync4 frontend blueprint for a frontend that uses the current LQoSync3 backend.
Continue the earlier brainstorm. Use Vercel and Google frontend guidance.
Do not create an LQoSync4 branch or project directory to read its documents.
The request covers research and discussion. It does not select a final design or authorize implementation.

## Evidence

Read remote documents through the GitHub connector. No checkout or temporary project directory was necessary.
The remote `lqosync4` head was `b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8` at the checkpoint.

- [Roadmap](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/ROADMAP.md)

- [Routes](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/ROUTES_AND_COMPONENTS.md)

- [Visual system](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/UI_VISUAL_SYSTEM.md)

- [Rust frontend options](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/RUST_FRONTEND_OPTIONS.md)

- [Product specification](https://github.com/p33ckab00/LQoSync/blob/b5284bd9fc4c3e8d7a35b8c6f24e2e4faf8709e8/docs/lqosync4/docs/PRODUCT_SPEC.md)

- Also read the feature map, decision ledger, and first 70 lines of `ui-reference.css`.

The remote documents describe a planned product. They do not prove a working web application.
Their accepted Leptos CSR decision belongs to that product. Select the framework for this frontend before implementation.
Their TUI removal and RouterOS writer scope do not override this session's request.

Local evidence: `src/tui/app.rs`, `src/tui/data.rs`, `src/api/dispatch.rs`, `src/api/server.rs`, and `Cargo.toml`.
The graph query supplied leads. Current files confirmed the important claims.
Older handoff and status notes contain incomplete feature lists.

## Reuse map

| Blueprint item | Proposed LQoSync3 use | Limit |
|---|---|---|
| Dark sidebar, slim top bar, page header | Shared shell with data age and service state | Dimensions remain starting values |
| Subscriber and assignment tables | Search, filters, paging, stable IDs, detail drawer | Browser API and safe DTOs remain work |
| Allocation cards | WAN capacity and shaping-tree placement | Placement does not prove MikroTik routing |
| Profile cards | LQoSync-owned plans and effective speed | Do not imply RouterOS profile writes |
| LibreQoS panel | Candidate, written, imported, and observed result | Show success only when evidence supports it |
| Mobile drawer and record cards | Reachable actions on narrow screens | Verify focus, keyboard, zoom, and overflow later |
| Loading and error states | Keep last values with age and stale state | Unknown is not zero or healthy |

## Recommended first information structure

Start with the eight labels in the current navigation:
Dashboard, Subscribers, Routers, Topology, LibreQoS, Workers, Settings, and Journal.
Keep PPPoE, DHCP, and Hotspot as Subscriber filters initially.
Separate IPoE and PPPoE pages only when their workflows require separate navigation.
Place plans under the related subscriber or topology workflow initially.
Do not expose planned routing actions as working controls.

The Dashboard must answer four questions: Is synchronization healthy? How old is the data?
Which subscribers have problems? What changed in the latest cycle?

Use an issue list with links, a bounded traffic panel, and a compact WAN table.
Charts need actual samples. Missing samples must not become a zero line.

## Backend boundary

The current API uses JSON lines through `<state_dir>/api.sock` with mode `0600`.
Its peer rule accepts root or the service UID. It is not a browser HTTP API.
Dispatch includes status, registry, topology tree/preview/policies, WAN discovery, edit proposals, plans, and subscriber plan assignment.
The TUI also reads status files, snapshots, and telemetry. These are not all socket operations.

Proposed path: browser UI -> authenticated Rust HTTP adapter -> existing service API and safe snapshot readers.
The service retains validation, revisions, collection, and publication ownership.
Do not expose raw files, credentials, or the socket directly to browser clients.
Resolve adapter privileges, sessions, CSRF protection, and DTO redaction before implementation.
Cached page reads must not start a router collection.

Preserve each operation's existing confirmation rules and publication semantics.
Do not add a universal Apply step from the older blueprint.
Show saved policy separately from published and observed shaping state.

## Design approaches for discussion

| Approach | Benefit | Cost |
|---|---|---|
| Adapt LQoSync4 visuals to LQoSync3 workflows | Reuses the blueprint and current domain operations | Requires explicit capability mapping |
| Copy the current TUI structure closely | Familiar navigation and smallest initial scope | Some terminal interaction patterns need web changes |
| Deliver the complete LQoSync4 product | Broad router and routing controls | Requires substantial backend work beyond this request |

Recommend the first approach. Reuse the second approach's navigation for the first slice.
Propose read views first: shell, Dashboard, Subscribers, and LibreQoS status.
Add topology edits only with version conflicts, confirmation, and result states.

## Visual starting values

Keep the source palette: canvas `#0b0c0e`, surface `#17191b`, text `#f1f3f5`, accent `#80aaff`.
Reserve green `#2bd391`, amber `#ffd077`, and red `#ff858a` for labeled states.
Use a sans-serif face for UI text and tabular numerals for rates and counts.
Start with 14–16 px body text, 24 px page padding, and 16 px panel gaps.

Use a desktop sidebar and a phone drawer. Keep dense tables within their own scroll panels.
Validate contrast, 200% zoom, 320 px width, focus return, and draft retention after implementation.

## External guidance and open questions

- Read the official [Vercel skill](https://github.com/vercel-labs/agent-skills/blob/main/skills/web-design-guidelines/SKILL.md) and [Web Interface Guidelines](https://vercel.com/design/guidelines).

- Use keyboard access, visible focus, persistent errors, and complete operational states as design criteria.

- No local Vercel or Google frontend skill was found in the shared skill catalog.

- The owner identified GoogleChrome `modern-web-guidance`. Material Design 3 and Stitch are not the requested Google skill.

- [Google adaptive navigation guidance](https://developer.android.com/design/ui/mobile/guides/layout-and-content/layout-and-nav-patterns) supplies general layout ideas. Its code targets Android, not Rust web UI.

- [Google Stitch skills](https://github.com/google-labs-code/stitch-skills) require a Stitch MCP connection. No Stitch tools are available in this session.

- No complete local web brainstorm document was found by the targeted search. Existing TUI designs and service API decisions provide continuity.

## Verification and next action

Only research notes changed. No source, dependencies, services, production data, or branch changed.

No build, application test, or browser verification occurred because no application code changed.
Existing untracked operational notes remain untouched.
Headroom discovery succeeded, but its proxy was unreachable. No compression savings occurred.
Next: review the proposed navigation and Dashboard content with the owner.


## Google guidance clarification

The owner supplied [GoogleChrome/modern-web-guidance](https://github.com/GoogleChrome/modern-web-guidance).
Read its skill at revision `64a6d708f5a43f2cd3fde5d4d7db3905ba1bdf65`.
Used the temporary CLI through `npx`. No persistent skill installation or project dependency changed.
One concurrent CLI call failed because generated guide data was absent. The other calls succeeded.
Search results identified `responsive-table`, `navigation-drawer`, and `dialog`. Retrieved and read all three guides.

Proposals for the frontend:

- Use semantic tables with named, keyboard-accessible scroll regions. Keep headers and subscriber identity visible during scroll.
- Adapt tables to their panel width with container queries. Preserve labels and header relationships in narrow layouts.
- Choose drawer semantics deliberately. A desktop sidebar must not retain a modal dialog role.
- Prefer native modal dialogs for confirmations. Set initial focus to the safe action for destructive changes.
- Keep unsaved edits safe on dismissal. Do not infer a new confirmation from a previous dialog result.
- Treat recent browser features as enhancements with the prescribed fallbacks. Check actual target browsers before implementation.
- Adapt browser API examples to Rust/Wasm bindings if the owner selects that stack.

These are research proposals. No application code or browser behavior was verified.
Vercel supplies interaction review criteria. Google guidance supplies platform patterns and compatibility checks.
The LQoSync4 blueprint supplies layout ideas. Current LQoSync3 source defines operational capabilities.


## Exact Vercel references from the owner

The owner supplied these sources after the Google clarification:

- [Open Agents frontend-design](https://github.com/vercel-labs/open-agents/blob/main/.agents/skills/frontend-design/SKILL.md).
- [Vercel Agent Skills](https://github.com/vercel-labs/agent-skills).

Read the exact frontend skill, the Agent Skills catalog, `web-design-guidelines`, and its current rule source.
The frontend skill guides aesthetic direction, typography, color, composition, and motion.
The review skill checks accessibility, forms, navigation state, performance, and feedback.
React and Next.js skills apply if the owner selects that stack. They do not establish a framework decision.
No skill installation, dependency change, or deployment occurred.

Proposed aesthetic: a precise network operations console with controlled density.
Keep the dark blueprint palette, aligned rates, clear subscriber identities, and explicit data age.
Consider IBM Plex Sans for UI text and IBM Plex Mono for technical identifiers. Font choice remains a proposal.
Use the topology hierarchy as the distinctive visual element, with restrained surrounding panels.
Reserve motion for changes that help an operator understand an action or state transition.
Evaluate source recommendations against operator tasks, accessibility, and the selected framework.
The research has not selected final tokens, fonts, navigation, or interaction contracts.


## Picture mockups for owner review

The owner requested pictures of the screens described in the LQoSync4 documents.
Generated 11 desktop preview images with the built-in imagegen tool.
Screens: Dashboard, Routers, IPoE Subscribers, IPoE Profiles, PPPoE Users, PPPoE Profiles, LibreQoS, Load Balancer Allocations, Load Balancer Assignments, Users, and Settings.
The previews use sample data and the source document's dark layout.
They include proposed LQoSync4 capabilities. They do not prove current LQoSync3 support.
Generated text, counts, dates, and chart values contain inconsistencies. Treat them as visual concepts, not approved data contracts.
Discovery and History appear as Load Balancer tabs. Their separate tab bodies and mobile variants were not generated.
Images remain at `/home/pi/.codex/generated_images/01a1f7d8-1525-7e03-91da-3fda9b62560c/` and were displayed in chat.
No application source, branch, runtime state, or dependency changed.
Next: obtain the owner's visual feedback before a final design or implementation.
