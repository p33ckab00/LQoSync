# Config Guidance + Role-Aware Navigation

v2.70.12-rc1 keeps `config.json` as the runtime source of truth while making advanced configuration safer to understand and lower-role navigation less misleading.

## What changed

- Added one shared config-guide registry used by both the WebUI Advanced JSON inspector and bundled documentation.
- Advanced JSON now opens with a searchable field inspector that answers **What / Why / When / Who / Where / How**, plus default, risk, and related paths.
- Added `docs/content/config_field_guide.md`, generated from the same registry, so installers and operators see the same explanations outside the WebUI.
- Hid admin-only `Lifecycle` and `Reports` sidebar links from operator/viewer roles while keeping backend route guards authoritative.
- Aligned role descriptions with the real route model: operator/viewer roles no longer claim access to admin-only reports/lifecycle pages.
- Extended UI Wiring Audit to verify both admin-only sidebar hiding and shared config-guide wiring.

## Operator meaning

```text
config.json
   ↓
shared guide registry
   ├─ WebUI Advanced JSON inspector
   └─ install/operator documentation
```

The UI is now calmer without lying. Lower roles only see sidebar destinations they can actually open, while direct-route protection still comes from Flask role guards. Admins and owners retain the full JSON truth view, but it is now accompanied by the same installation-grade guidance available in the documentation center.

## Safety notes

- The raw JSON remains strict JSON; explanations live beside it, not inside it.
- `network_mode`, `flat_network`, and `no_parent` keep their existing topology semantics.
- Routers and generated network format are not simplified or refactored by this release.
- This release changes guidance and visibility only; it does not change collection, cleanup decisions, generated files, scheduler cadence, or LibreQoS apply behavior.
