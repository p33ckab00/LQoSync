# Rust Core Live Read Adapter Pilot

LQoSync `lqosync-in-rust` exposes `run-routeros-live-read-adapter-pilot`.

This phase bridges the fixture-only authenticated read pipeline and production
collector parity. Contract mode still performs no network I/O. Live mode can
execute a single read-only RouterOS API `print` command only when every
live-read gate is explicitly enabled.

## Safety status

This is **not** full Rust backend yet.

The operation does not write RouterOS config, does not write LibreQoS files, and
does not transfer cleanup or collector authority. Python collectors remain
authoritative until live-read shadow parity gates pass.

Live execution requires:

```text
allow_rust_routeros_live_reads=true
allow_rust_routeros_credentials=true
allow_rust_routeros_tcp_connect=true
allow_rust_routeros_live_read_adapter=true
routeros_live_read_pilot=true
routeros_live_read_adapter_pilot=true
routeros_transport_authority=live_read_adapter_pilot
```

## Operation

```text
run-routeros-live-read-adapter-pilot
```

Expected safe contract result:

```text
live_read_adapter_contract_ready
connection_attempt_count=0
authentication_attempt_count=0
api_sentence_write_count=0
api_reply_read_count=0
```

Expected gated live-read result:

```text
live_read_adapter_read_complete
connection_attempt_count=1
authentication_attempt_count=1
collector_authority=python_authoritative
safe_for_cleanup=false
```

## API

```text
GET  /api/rust-core/routeros-live-read-adapter-pilot
POST /api/rust-core/routeros-live-read-adapter-pilot
```

Example:

```bash
curl -X POST http://YOUR-LQOSYNC/api/rust-core/routeros-live-read-adapter-pilot \
  -H 'Content-Type: application/json' \
  -d '{
    "router": {"name":"R1","address":"10.0.0.1","username":"admin","password":"secret"},
    "adapter":"contract",
    "mode":"contract",
    "execute":false,
    "fixture_reply_words":["!done"],
    "path":"/ppp/active",
    "fields":["name","address"]
  }'
```

## Config flags

```json
{
  "rust_core": {
    "routeros_transport_authority": "plan_only",
    "allow_rust_routeros_live_reads": false,
    "allow_rust_routeros_credentials": false,
    "allow_rust_routeros_tcp_connect": false,
    "allow_rust_routeros_live_read_adapter": false,
    "routeros_live_read_pilot": false,
    "routeros_live_read_adapter_pilot": false,
    "routeros_live_read_adapter_authority": "contract_only"
  }
}
```

These flags are intentionally conservative. Set `routeros_transport_authority`
to `live_read_adapter_pilot` only for a read-only pilot window, then return it
to `plan_only` after the pilot run.

## Next phase

Successful live-read results can now feed
`build-routeros-live-read-shadow-parity`, which builds Rust shadow collector
rows and compares them with Python collector rows. The next step is repeated
run-cycle history before any authority handoff.
