# Rust Core v2.1 RouterOS Read Result Contract

Rust Core v2.1 adds command-level validation for RouterOS read results.

This is the next bridge after v2.0 RouterOS collector planning. Rust still does
not connect to MikroTik. Python continues to perform live RouterOS API reads.
Rust now validates whether the returned command results match the deterministic
read plan before those rows are trusted for cleanup/circuit processing.

## Operation

```text
validate-routeros-read-results
```

## Input

```json
{
  "plan": {"commands": []},
  "results": [
    {
      "router": "RB5k9-Distro",
      "source": "pppoe",
      "path": "/ppp/active",
      "status": "ok",
      "rows": [],
      "duration_ms": 12.5
    }
  ],
  "previous_counts": {},
  "slow_ms_threshold": 2000,
  "strict": false
}
```

## Output

The response includes:

- planned command count
- received result count
- missing required reads
- failed reads
- suspicious zero-result detection
- slow read warnings
- per-source `safe_for_cleanup` signals
- row snapshots grouped by router/source/path

## Safety model

This is still not a full Rust RouterOS backend.

```text
Python performs live RouterOS API reads.
Rust validates the result contract.
Python remains authoritative by default.
Rust live transport is not implemented yet.
```

A result is not trusted when required planned reads are missing, a command reports
failed/timeout/partial, or a required result returns zero rows after previous
non-zero success without being marked `zero_valid`.

## API endpoint

```text
POST /api/rust-core/routeros-read-results
```

The endpoint is diagnostic and does not mutate config, state, LibreQoS files, or
MikroTik state.
