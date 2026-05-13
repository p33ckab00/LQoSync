# Policy Settings Integration

LQoSync v2.49 makes Smart Policy Center a real settings surface. Policies are operator intent stored in `config.json -> policies`, not hidden backend behavior.

## Source of truth

- `config.json -> policies`: operator-configured policy values
- `engine/policy_schema.py`: labels, descriptions, allowed values, risk levels, defaults, and UI metadata
- `engine/policy_defaults.py`: base defaults and presets
- `engine/policy_engine.py`: runtime decision maker before write/apply
- `policy_state.json`: pending confirmations, cleanup queue, and runtime decision history

## Preset behavior

Available presets:

- Conservative
- Balanced
- Aggressive
- Custom

Applying a preset writes the full preset into `config.json -> policies`.

Manual edits from Policy Center or Config Center switch `policies.mode` to `custom` because the current values no longer exactly match a preset.

## Visible policy groups

- Cleanup Core
- PPPoE Cleanup
- DHCP Cleanup
- Hotspot Cleanup
- Static/manual Cleanup
- Mass Removal Guards
- Apply Guards
- Collector Guards
- Data Quality Guards
- Topology Guards
- Backup Guards
- Anomaly Detection
- Recommendations

## Operator workflow

1. Open Policy Center.
2. Choose a preset or edit individual policies.
3. Save custom policy settings.
4. Run Dry Run.
5. Review policy verdict, risk, confirmations, and recommendations.
6. Enable scheduler/auto-apply only when behavior is expected.

## Why this matters

Smart behavior must not be blind. Operators need to know what policy is enabled, what threshold is active, and what action happens when the policy is triggered.
