# LQoSync3

LQoSync3 reads subscriber data from MikroTik routers and writes shaping input for LibreQoS.
It replaces `/opt/libreqos/src/updatecsv.py`. It reads its settings from one TOML file.
It supports LibreQoS 2.0 to 2.2 and LibreQoS 3.x.

> [!warning] Status: LibreQoS 2.x in shadow testing
> The binary works for LibreQoS 2.x (`check`, `once`, `run`, `diff`, `defaults`). LibreQoS v3, Telegram, the TUI, and `install.sh` are not done yet.
> Run in `shadow` mode and compare with `lqosync3 diff` before you use `apply` on a production shaper.

## What the tool does

1. It reads PPPoE, DHCP, and Hotspot data from each router through the RouterOS API (`mikrotik-rs`).
2. It converts the data to one internal model of circuits and nodes.
3. It validates the model.
4. It writes the output format that the installed LibreQoS version reads.
5. It writes files only when the content changes.

## Output targets

| Target | LibreQoS | Files | Reload |
|--------|----------|-------|--------|
| `v2_pair` | 2.0 to 2.2 | `ShapedDevices.csv` + `network.json` | `LibreQoS.py --updateonly` |
| `v3_network` | 3.x, `[topology] source = "libreqos_network"` | `libreqos_network.json` (schema 2) | `lqosd` file watcher |
| `v3_pair` | 3.x, `[topology] source = "shaped_devices"` | `ShapedDevices.csv` + `network.json` | `lqosd` file watcher |

The default target is `auto`. The tool reads the installed LibreQoS version and the v3 topology source. Then it selects the target.

## Documents

- [Architecture](docs/ARCHITECTURE.md): parts, data flow, and performance rules.
- [Configuration](docs/CONFIGURATION.md): the TOML reference and the map from `config.json`.
- [Installation](docs/INSTALLATION.md): the one-line installer and the manual procedure.
- [Operations](docs/OPERATIONS.md): daily tasks, cutover, LibreQoS upgrade, rollback, and troubleshooting.
- [LibreQoS v3 ingress](docs/LIBREQOS-V3-INGRESS.md): the `libreqos_network.json` schema and the offline simulation results.

## Project memory

- `brain/` is the Obsidian vault for decisions, session notes, and knowledge. Start at [brain/Home.md](brain/Home.md).
- `graphify-out/` holds the knowledge graph of the documents and the vault.
- `brain/HANDOFF.md` holds the current state for the next agent or operator.

## Safety rules

- The installer starts the tool in `shadow` mode. In `shadow` mode, the tool does not write LibreQoS files.
- The tool does not stop `lqosync-updatecsv.service` (the Python service). The operator does the cutover.
- The tool keeps router credentials in files with mode `0600`. Do not put passwords in the TOML file.
- Router writes are not part of this tool. The tool only reads from routers.
