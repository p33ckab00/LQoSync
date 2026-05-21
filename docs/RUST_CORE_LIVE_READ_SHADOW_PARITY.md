# Rust Core Live-Read Shadow Parity

`build-routeros-live-read-shadow-parity` is the bridge after the gated
RouterOS live-read adapter pilot.

It turns supplied or freshly gated live-read results into the existing Rust
RouterOS shadow collector bundle, then compares the generated rows with Python
authoritative collector rows when `python_rows` is supplied.

## Safety Status

This is still shadow-only:

```text
collector_authority=python_authoritative
safe_for_cleanup=false
write_allowed=false
apply_allowed=false
```

Rust may build parity evidence, but Rust output cannot drive cleanup, generated
file writes, LibreQoS apply, or production collector authority from this
operation.

## Operation

```text
build-routeros-live-read-shadow-parity
```

Contract/default mode performs no network I/O. Live mode is only possible
through `run-routeros-live-read-adapter-pilot` gates, or by supplying prior
live-read results:

```json
{
  "live_read_adapter": {
    "result": {
      "status": "live_read_adapter_read_complete",
      "read_result": {
        "router": "RB5009",
        "source": "pppoe",
        "path": "/ppp/active",
        "status": "ok",
        "rows": []
      }
    }
  },
  "live_read_results": []
}
```

The operation also accepts the same raw `results` or `read_results` shape used
by `build-routeros-shadow-collector-bundle`.

## API

```text
GET  /api/rust-core/routeros-live-read-shadow-parity
POST /api/rust-core/routeros-live-read-shadow-parity
```

Expected successful parity status:

```text
live_read_shadow_parity_pass
```

Expected review status when Python rows are not supplied or differ:

```text
live_read_shadow_parity_review
```

Expected blocked status when required RouterOS reads are missing:

```text
blocked
```

## Next Phase

Wire this parity evidence into the run-cycle shadow history so operators can
require repeated clean cycles before any collector authority handoff.
