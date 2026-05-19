# Collector Output Contract

This document defines the safety contract between MikroTik collectors and the Rust core.

The goal is to prevent silent collector failures from becoming destructive cleanup decisions.

## Why this contract exists

A source can fail without throwing an exception. For example, a RouterOS API call may return an empty list even though the source previously had active clients. If that empty list is treated as a successful scan, cleanup policy may remove active clients from `ShapedDevices.csv`.

The collector contract makes source trust explicit.

```text
collector output is not trusted until Rust validates it
```

## Required collector envelope

Every collector result should be wrapped before diff/cleanup sees it.

```json
{
  "router": "RB5009-Core",
  "source": "pppoe",
  "status": "ok",
  "rows": [],
  "expected_reads": ["/ppp/active", "/ppp/secret", "/ppp/profile"],
  "successful_reads": ["/ppp/active", "/ppp/secret", "/ppp/profile"],
  "failed_reads": [],
  "read_counts": {
    "active": 20,
    "secrets": 45,
    "profiles": 4
  },
  "previous_counts": {
    "active": 18,
    "generated_rows": 18
  },
  "errors": [],
  "warnings": [],
  "safe_for_cleanup": true,
  "safe_for_diff": true,
  "safe_for_write": true
}
```

## Status values

```text
ok              complete and trusted result
partial         at least one expected read failed or is missing
failed          collector failed hard
zero_valid      zero rows is expected and safe
zero_suspicious zero rows is unexpected based on prior successful state
unknown         malformed envelope or unclassified state
```

## Source names

Use normalized source names:

```text
pppoe
dhcp
hotspot
static
```

Display labels may remain:

```text
PPP
DHCP
HS
STATIC
```

## Cleanup safety rules

Rust should calculate these booleans:

```text
safe_for_cleanup
safe_for_diff
safe_for_write
safe_for_apply
```

Recommended rules:

| Condition | safe_for_cleanup | Notes |
|---|---:|---|
| All expected reads succeeded, non-zero or expected zero | true | Normal trusted source. |
| One or more expected reads failed | false | Preserve existing rows. |
| Collector exception | false | Preserve existing rows. |
| Empty result after previous non-zero success | false | Mark `zero_suspicious`. |
| Source disabled intentionally | false by collector, policy decides later | Do not let collector delete rows directly. |
| Static/manual rows | false by default | Operator-owned rows should not be removed by source scan. |

## Example: suspicious zero PPPoE result

Request:

```json
{
  "version": "1",
  "op": "validate-collector-output",
  "payload": {
    "router": "RB5009-Core",
    "source": "pppoe",
    "status": "ok",
    "rows": [],
    "expected_reads": ["/ppp/active", "/ppp/secret", "/ppp/profile"],
    "successful_reads": ["/ppp/active", "/ppp/secret", "/ppp/profile"],
    "failed_reads": [],
    "read_counts": {
      "active": 0,
      "secrets": 0,
      "profiles": 0
    },
    "previous_counts": {
      "active": 22,
      "generated_rows": 22
    }
  }
}
```

Response:

```json
{
  "version": "1",
  "op": "validate-collector-output",
  "ok": false,
  "result": {
    "status": "zero_suspicious",
    "safe_for_cleanup": false,
    "safe_for_diff": true,
    "safe_for_write": false,
    "safe_for_apply": false
  },
  "errors": [],
  "warnings": [
    {
      "code": "collector_zero_suspicious",
      "severity": "warning",
      "message": "PPPoE returned zero rows after a previous successful non-zero result.",
      "safe_for_cleanup": false
    }
  ],
  "meta": {
    "engine": "lqosync-core"
  }
}
```

## Integration point in run cycle

The contract must run before these steps:

```text
cleanup candidate building
diff generation
file write
LibreQoS apply decision
```

Recommended flow:

```text
Python collector reads RouterOS
  ↓
Python wraps raw output in collector envelope
  ↓
Rust validates envelope
  ↓
Rust returns trust verdict
  ↓
cleanup_sources only includes source when safe_for_cleanup=true
```

## Cache relationship

Collector validation may use previous counts from `collector_cache.json` or runtime state. Because this influences cleanup safety, `collector_cache.json` must be written atomically in the same safety family as `runtime_state.json` and `policy_state.json`.

## Current Rust support

The Rust scaffold includes `validate-collector-output` now, even before a full
collector rewrite. This allows Python collectors to pass a trust envelope before
cleanup/diff/write decisions rely on their output.

Example request:

```json
{
  "version": "1",
  "op": "validate-collector-output",
  "payload": {
    "router": "RB5009-Core",
    "source": "pppoe",
    "status": "ok",
    "rows": [],
    "failed_reads": [],
    "previous_success_count": 18
  }
}
```

If `rows` is zero after a previous successful non-zero run and status is not
`zero_valid`, Rust returns `safe_for_cleanup=false` with a
`collector_zero_suspicious` diagnostic.
