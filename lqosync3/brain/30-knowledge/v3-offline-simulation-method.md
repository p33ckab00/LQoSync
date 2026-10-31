---
type: knowledge
created: 2026-10-31
tags: [libreqos-v3, testing]
status: draft
related: ["[[libreqos-v3-ingress-facts]]"]
---

# Offline simulation of the LibreQoS v3 compiler

Use this method to test a v3 input file without a v3 installation.

## Procedure

1. Get the package file name from the beta index:
   `https://packages.libreqos.com/dists/beta/main/binary-amd64/Packages`.
2. Download the `.deb` to a new empty directory. Compare the SHA-256 value with the index.
3. Extract it with `dpkg-deb -x`. Do not use `dpkg -i` or `apt install`.
4. Copy `opt/libreqos/src/liblqos_python.so` to a separate module directory.
5. Copy `lqos.example` to a test `lqos.conf`. Set `lqos_directory` and `state_directory` to scratch directories. Set `[topology] source`.
6. Run Python with `env -i`, `LQOS_CONFIG=<test lqos.conf>`, and `python3 -I`.
7. Run the process under `strace -f -e inject=connect:error=EACCES`. This blocks all socket connections.
8. Call `write_compiled_topology_from_network_json_payload("libreqos_network", "full", json_text)`.
9. Read `state/shaping/topology_compiled_shaping.json` and `state/topology/topology_import.json`.

## Safety

- On the production shaper, `/run/lqos/bus` has mode `0777`. A connection reaches the production `lqosd`.
- Do not call `validate_shaped_devices`. It sends a request on the bus.
- Do not run the v3 `lqosd`. It changes network interfaces and queues.
- Examine the `strace` log after each run. Make sure that no write occurs outside the scratch directory.

## Limits

The method tests the parser and the compiler. It does not test the `lqosd` watcher, the last-good behavior, or the shaping result.
