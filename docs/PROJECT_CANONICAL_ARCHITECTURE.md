# Project Canonical Architecture

LQoSync is a local appliance-style web application that gives an interface to a MikroTik-to-LibreQoS synchronization process.

It is not a SaaS platform, not multi-tenant, and not a Django project.

## Final direction

```text
Rust = backend authority
Svelte = operator UI
LibreQoS = external target/middlebox
MikroTik = data source
```

## Rust daemon

`lqosync-core.service` is the single Rust backend service. It listens on:

```text
/run/lqosync-core.sock
0.0.0.0:9202
```

It owns scheduler authority, validation, sync planning, file mutation, transaction journaling, LibreQoS apply, recovery, rollback, quarantine, stability gates, the HTTP/API surface, and serving the embedded Svelte console.

## Svelte Operator Console

The operator UI is built with Svelte and served by `lqosync-core`. The supported runtime does not install or start a Python Flask backend service.

Legacy Python files may remain as historical tooling or migration support, but they are not the backend runtime.

## MikroTik sources

The system collects and normalizes:

- PPPoE active/secret/profile data
- DHCP/IPoE leases and Option82-style identity where available
- Hotspot users/active sessions
- manual/static mappings

## LibreQoS outputs

Rust authority writes:

- `ShapedDevices.csv`
- `network.json`

Rust authority can execute:

```bash
LibreQoS.py --updateonly
```
