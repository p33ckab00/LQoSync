# Rust Core v1.0 Apply Transaction Executor

Rust Core v1.0 adds the `execute-apply-transaction` protocol operation.

This is the first opt-in executor for the safe file-write portion of the apply
manifest introduced in v0.9. It is intentionally conservative:

- default behavior is rehearsal only;
- Python remains authoritative for the normal sync/write/apply path;
- Rust does not run `LibreQoS.py` in v1.0;
- file writes require both `execute=true` and `allow_file_writes=true`;
- config defaults keep execution disabled.

## Config flags

```json
"rust_core": {
  "transaction_authority": "preview",
  "execute_apply_manifest": false,
  "allow_rust_file_writes": false,
  "allow_rust_libreqos_apply": false
}
```

## Operation

```json
{
  "version": "1",
  "op": "execute-apply-transaction",
  "payload": {
    "execute": false,
    "allow_file_writes": false
  }
}
```

When execution is disabled, the operation returns `rehearsal_only` and writes
nothing. When explicitly enabled and the manifest status is `ready`, Rust can
atomically write generated `ShapedDevices.csv` and/or `network.json` using the
same checksum-protected writer introduced in v0.3.

## Safety notes

Do not enable Rust file-write execution in production until Dry Run parity has
been observed for multiple cycles. This feature exists to prepare for future
Rust transaction authority while preserving current Python behavior.
