# Rust Core v3.4 Live Read Adapter Contract

LQoSync `2.104.0-rc1` adds `lqosync-core v3.4.0` with the `run-routeros-live-read-adapter-pilot` operation.

This phase is a bridge between the fixture-only authenticated read pipeline and a future live Rust RouterOS socket adapter. It builds a guarded live-read adapter contract by composing the TCP connectivity pilot, the redacted auth-session contract, and the RouterOS API sentence encoder.

## Safety status

This is **not** full Rust backend yet.

The operation does not open RouterOS sockets, authenticate to MikroTik, emit credentials, send API words, read API replies, replace Python collectors, or write LibreQoS files.

If `execute=true` or a live adapter/mode is requested, the Rust core returns `routeros_live_read_adapter_not_implemented`.

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
    "routeros_live_read_adapter_pilot": false,
    "allow_rust_routeros_live_read_adapter": false,
    "routeros_live_read_adapter_authority": "contract_only"
  }
}
```

These flags are intentionally conservative. A real live adapter remains blocked until a future phase implements the socket/auth/read state machine.

## Next phase

The next bridge is expected to introduce a read-only live adapter implementation path behind explicit gates, while Python collectors remain the fallback and authority until parity is proven.
