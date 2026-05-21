# Project Canonical Architecture

LQoSync is a local appliance-style web application that gives an interface to a MikroTik-to-LibreQoS synchronization process.

It is not a SaaS platform, not multi-tenant, and not a Django project.

## Final direction

```text
Rust = backend authority
Python Flask = WebUI shell only
LibreQoS = external target/middlebox
MikroTik = data source
```

## Rust daemon

`lqosync-core.service` is the single Rust authority daemon. It listens on:

```text
/run/lqosync-core.sock
```

It owns scheduler authority, validation, sync planning, file mutation, transaction journaling, LibreQoS apply, recovery, rollback, quarantine, and stability gates.

## Flask WebUI shell

The existing Flask UI remains because it is already the operator interface. It should not be rewritten to Django. It should not regain scheduler or mutation authority.

Flask calls Rust and displays the result.

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
