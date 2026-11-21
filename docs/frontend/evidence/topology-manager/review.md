# Topology web design review

Date: 2026-11-21, Asia/Shanghai.
Scope: documents, source mapping, static mockups, and implementation gates.
Result: the package is ready for owner review. Production implementation and runtime acceptance remain future work.

## Feature coverage

| Feature IDs | Planned delivery |
|---|---|
| TM-01 | T00 coherent view, T01 adapter, T02 Builder |
| TM-02, TM-03, TM-09 | T07 explicit refresh/discovery/source jobs after T05 |
| TM-04–TM-08 | T06 Builder forms and service impact review |
| TM-10 | T03 assignment reads, with offline records separate from current endpoints |
| TM-11, TM-12 | T08 pins and scoped rebalance |
| TM-13 | T08 plans, identity selection, and aggregate preflight |
| TM-14–TM-16 | T09 speed rules, source factors, and fallback |
| TM-17, TM-18 | T10 conflicts, audit limits, and correlated publication evidence |
| TM-19 | Deferred schedule backend, with explicit unavailable state |
| TM-20 | Read prerequisites only. Controlled CLI actions stay outside the initial web scope |
| TM-21 | Deferred router routing/DPI writer. Placement remains distinct |
| TM-22 | Read domain values only until narrow service operations exist |
| TM-23, TM-24 | T00 observation/identity projection, T01 adapter, T03 PPPoE Users and IPoE Clients |
| TM-25, TM-26 | T03 shared plan membership/details, T08 common actions and cross-source review |

The reliability gates T00, T04, and T05 precede mutation UI delivery.
T11 supplies runtime/browser/recovery release proof. Its tasks are unchecked.

## Findings and resolution

| Finding | Resolution in this package |
|---|---|
| Older notes describe incomplete TUI phases | Current dispatch, Runner, TUI forms, and domain code supply the feature map |
| TUI refresh-on-open can read routers | Web GET stays cached. Explicit POST jobs own refresh/discovery |
| Independent reads can mix revisions | A new bounded Runner-owned snapshot contract precedes the UI |
| Preview arrays cover active observations only | Offline pins require a separate projection; old endpoints never become current |
| One global proposal can be replaced by another client | Broker lease, principal-bound opaque IDs, expiry, and local replacement tests |
| Lost replies can duplicate writes | Durable core request-key receipts and lookup precede web mutations |
| Plan methods lack the Builder's impact proposal | Require service aggregate preflight or a service-generated review |
| Generic client checks only part of the response envelope | Add version, ID, shape, and size checks to the new adapter |
| Saved policy does not prove native acceptance | Separate saved revision, file/import phase, and correlated native evidence |
| Current Changes view is not complete history | Add bounded web history and disclose absent local actor coverage |
| Full service replies can exceed transport limits | Bound and page in the core projection before serialization |
| A tree can double-count members | Distinguish subtree summaries from selected-node direct members |
| Mockup budgets can imply impossible policy | Use documented 2:1 capacity arithmetic and a separate conflict fixture |
| A static overlay can imply tested modal behavior | Mark it as a picture. Require native dialog tests in production |
| Claude Code still edits the shared worktree | Use the owner-approved work split and publish only isolated Topology documentation paths |

## Completed verification

The static capture script passed 25 scenes with local fonts and no horizontal page overflow.
It checked the 13px text floor and the declared text/control/focus contrast thresholds.
The images include desktop, tablet, narrow, light, dark, conflict, and review states.
The available Chromium renderer supplied these checks. No cross-browser runtime acceptance claim follows.

Manual visual review checked Builder, compact Builder, Policies, Changes, and the review panel.
Manual source review checked operation names, proposal lifetime, revision behavior, capacity rules, and writer ownership.
The document review checked feature coverage, future API labels, dependency order, deferred controls, and exact sample arithmetic.

The STE checker is heuristic. Technical term flags require manual review.
The documents do not claim certified ASD-STE100 compliance.
No runtime code, production configuration, router state, or service state changed as part of this task.

## Remaining conditions

The owner must review the design before implementation starts.
Future implementation must pass TQ-01 through TQ-15 and all task-specific checks.
The owner later authorized publication after review of the no-collision work-split note.
Only the isolated Topology documentation paths form part of this commit.
No process termination, staging of unrelated files, or forced Git update forms part of this task.

## Subscriber-view extension

The same delivery now includes PPPoE Users, IPoE Clients, and both source Local plans views.
Read the [subscriber-view contract](../../TOPOLOGY_SUBSCRIBER_VIEWS.md).
The feature map has 26 features. Existing reliability gates still apply to every entry route.

Four new editable HTML scenes supply 12 additional captures.
Desktop, light, mobile, and 320px source views passed the static capture checks.
Both plan scenes passed desktop and mobile checks.
Manual visual review confirmed source states, shared subscriber identity, local table scroll, and unique plan counts.
The source fixture has four unique plan members across five verified identities.
A historical DHCP row carries no current endpoint or verified policy link.

STE review corrected long paragraphs and avoidable wording.
Technical names and explicit state labels remain exact. The checker is heuristic.
No production source file, service, router, or runtime test changed.
T00/T01 must add safe observation contracts before T03 source views can ship.
The T08/T11 cross-source integrity and recovery checks remain unchecked release conditions.
