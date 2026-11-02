#!/usr/bin/env python3
"""Reproduce the node rate loss of the LibreQoS 3.0.7 native topology compiler.

The script compiles one synthetic topology two times with the installed LibreQoS
Python binding:

- control: [topology] source = "shaped_devices" (network.json + ShapedDevices.csv)
- native:  [topology] source = "libreqos_network" (libreqos_network.json, schema 2)

Both inputs have the same nodes, rates and circuit. The script then reads the
compiled state files and compares the node kinds and node rates.

Safety:
- Each case has its own scratch folder, lqos.conf, input folder and state folder.
- Each compile runs under strace. All connect() calls fail with EACCES, so the
  binding cannot talk to a live lqosd. The script fails if a file is opened for
  write outside the scratch folder.
- The data is synthetic. It contains no subscriber data.

Usage: python3 repro.py [--out DIR] [--lib /opt/libreqos/src] [--modes full,full2,...]
Exit code: 0 if the native output keeps the node policy, 1 if the defect shows.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Synthetic topology: a site with one child node, and one circuit below the child node.
NODES = [
    {"id": "repro-site-a", "name": "SITE-A", "type": "site", "parent": None, "down": 200.0, "up": 100.0},
    {"id": "repro-node-a", "name": "PLAN-A", "type": "node", "parent": "repro-site-a", "down": 50.0, "up": 25.0},
]
CIRCUIT = {
    "id": "REPROC1",
    "name": "repro-circuit",
    "parent": "repro-node-a",
    "device_id": "REPROD1",
    "ip": "10.255.0.2",
    "mac": "02:00:00:00:00:01",
    "commit": (5.0, 2.5),
    "ceil": (50.0, 25.0),
}

CHILD = r"""
import json, os, sys
sys.path.insert(0, os.environ["REPRO_LIB"])
import liblqos_python as m
if os.environ["REPRO_SOURCE"] == "libreqos_network":
    text = open(os.environ["REPRO_INPUT"]).read()
    ok = m.write_compiled_topology_from_network_json_payload("libreqos_network", os.environ["REPRO_MODE"], text)
else:
    ok = m.write_compiled_topology_from_legacy_artifacts("shaped_devices", os.environ["REPRO_MODE"])
print("REPRO_RESULT " + json.dumps(ok))
"""


def native_input() -> dict:
    """libreqos_network.json, schema 2: the format that the LibreQoS 3.0.7 reader accepts."""
    rate = lambda n: {"commit_down_mbps": n["down"], "commit_up_mbps": n["up"],
                      "ceil_down_mbps": n["down"], "ceil_up_mbps": n["up"]}
    c = CIRCUIT
    return {
        "schema_version": 2,
        "match": {"mode": "ip"},
        "topology": [{"id": n["id"], "name": n["name"], "type": n["type"],
                      "parent": n["parent"], "rate": rate(n)} for n in NODES],
        "circuits": [{
            "id": c["id"], "display_name": c["name"], "parent": c["parent"],
            "plan": {"commit_down_mbps": c["commit"][0], "commit_up_mbps": c["commit"][1],
                     "ceil_down_mbps": c["ceil"][0], "ceil_up_mbps": c["ceil"][1]},
            "match": {"prefixes": [c["ip"] + "/32"]},
            "devices": [{"id": c["device_id"], "name": c["name"], "mac": c["mac"],
                         "ipv4": [c["ip"]], "ipv6": [], "comment": ""}],
        }],
    }


def pair_input() -> tuple[dict, str]:
    """network.json and ShapedDevices.csv with the same nodes, rates and circuit."""
    def node(n):
        kids = {k["name"]: node(k) for k in NODES if k["parent"] == n["id"]}
        return {"id": n["id"], "type": n["type"], "downloadBandwidthMbps": n["down"],
                "uploadBandwidthMbps": n["up"], "children": kids}
    net = {n["name"]: node(n) for n in NODES if n["parent"] is None}
    c = CIRCUIT
    parent = next(n for n in NODES if n["id"] == c["parent"])
    head = ("Circuit ID,Circuit Name,Device ID,Device Name,Parent Node,Parent Node ID,Anchor Node ID,"
            "MAC,IPv4,IPv6,Download Min Mbps,Upload Min Mbps,Download Max Mbps,Upload Max Mbps,Comment")
    row = (f'{c["id"]},{c["name"]},{c["device_id"]},{c["name"]},{parent["name"]},{parent["id"]},,'
           f'{c["mac"]},{c["ip"]},,{c["commit"][0]},{c["commit"][1]},{c["ceil"][0]},{c["ceil"][1]},')
    return net, head + "\n" + row + "\n"


def make_conf(example: str, case: Path, source: str, mode: str) -> Path:
    text = Path(example).read_text()
    text = re.sub(r'(?m)^lqos_directory = .*$', f'lqos_directory = "{case}/src"', text)
    text = re.sub(r'(?m)^state_directory = .*$', f'state_directory = "{case}/state"', text)
    text = re.sub(r'(?ms)^\[topology\]\n.*?(?=^\[)',
                  f'[topology]\nsource = "{source}"\ncompile_mode = "{mode}"\n\n', text)
    assert f'source = "{source}"' in text and f'{case}/state' in text, "lqos.conf rewrite failed"
    conf = case / "lqos.conf"
    conf.write_text(text)
    return conf


def run_case(root: Path, lib: str, example: str, source: str, mode: str) -> dict:
    case = root / f"{'native' if source == 'libreqos_network' else 'pair'}-{mode}"
    (case / "src").mkdir(parents=True)
    (case / "state").mkdir()
    if source == "libreqos_network":
        inp = case / "src" / "libreqos_network.json"
        inp.write_text(json.dumps(native_input(), indent=2))
    else:
        net, csv = pair_input()
        inp = case / "src" / "network.json"
        inp.write_text(json.dumps(net, indent=2))
        (case / "src" / "ShapedDevices.csv").write_text(csv)
    conf = make_conf(example, case, source, mode)
    trace = case / "strace.log"
    env = {"PATH": "/usr/bin:/bin", "LQOS_CONFIG": str(conf), "REPRO_LIB": lib,
           "REPRO_SOURCE": source, "REPRO_MODE": mode, "REPRO_INPUT": str(inp)}
    cmd = ["strace", "-f", "-qq", "-o", str(trace), "-e", "trace=connect,openat,rename,renameat,renameat2",
           "-e", "inject=connect:error=EACCES", sys.executable, "-I", "-c", CHILD]
    p = subprocess.run(cmd, cwd=case, env=env, capture_output=True, text=True, timeout=120)
    ok = None
    for line in p.stdout.splitlines():
        if line.startswith("REPRO_RESULT "):
            ok = json.loads(line.split(" ", 1)[1])
    return {"case": case, "source": source, "mode": mode, "ok": ok,
            "stderr": p.stderr.strip()[-800:], "outside_writes": outside_writes(trace, case)}


def outside_writes(trace: Path, case: Path) -> list[str]:
    """Files opened for write, or renamed, outside the case folder."""
    bad = []
    for line in trace.read_text().splitlines():
        if "= -1" in line:
            continue
        writes = "openat(" in line and re.search(r"O_WRONLY|O_RDWR|O_CREAT", line)
        if writes or "rename" in line:
            for path in re.findall(r'"(/[^"]*)"', line):
                if not path.startswith(str(case)) and not path.startswith("/dev/"):
                    bad.append(line.strip()[:200])
    return bad


def node_policy(case: Path) -> dict:
    """Node kind and rates: imported (input of the compiler) and compiled (its output)."""
    topo = case / "state" / "topology"
    out = {}
    imp = json.loads((topo / "topology_import.json").read_text())["imported"]
    can = json.loads((topo / "topology_canonical_state.json").read_text())
    native = imp.get("native_canonical") or {"nodes": []}
    for n in NODES:
        nid = n["id"]
        i = next((x for x in native["nodes"] if x["node_id"] == nid), None)
        c = next((x for x in can["nodes"] if x["node_id"] == nid), None)
        compat = find_compat(can.get("compatibility_network_json") or {}, nid)
        out[n["name"]] = {
            "input": {"type": n["type"], "down": n["down"], "up": n["up"]},
            "imported_native_canonical": i and {"kind": i.get("node_kind"), "rate_input": i.get("rate_input")},
            "compiled_canonical": c and {"kind": c.get("node_kind"), "rate_input": c.get("rate_input")},
            "compiled_compatibility": compat and {"type": compat.get("type"),
                                                  "down": compat.get("downloadBandwidthMbps"),
                                                  "up": compat.get("uploadBandwidthMbps")},
        }
    return out


def find_compat(tree: dict, node_id: str):
    for v in tree.values():
        if not isinstance(v, dict):
            continue
        if v.get("id") == node_id:
            return v
        hit = find_compat(v.get("children") or {}, node_id)
        if hit:
            return hit
    return None


def circuit_plan(case: Path) -> dict | None:
    p = case / "state" / "shaping" / "topology_compiled_shaping.json"
    if not p.exists():
        return None
    for d in json.loads(p.read_text()).get("shaped_devices", []):
        if d.get("circuit_id") == CIRCUIT["id"]:
            return {k: d.get(k) for k in ("parent_node_id", "download_min_mbps", "upload_min_mbps",
                                          "download_max_mbps", "upload_max_mbps")}
    return None


def policy_kept(policy: dict) -> bool:
    for n in NODES:
        c = policy[n["name"]]["compiled_compatibility"]
        if not c or float(c["down"] or 0) != n["down"] or float(c["up"] or 0) != n["up"]:
            return False
    return True


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", help="scratch folder (default: a new temporary folder)")
    ap.add_argument("--lib", default="/opt/libreqos/src", help="folder of liblqos_python.so")
    ap.add_argument("--example", default=None, help="lqos.conf template (default: <lib>/lqos.example)")
    ap.add_argument("--modes", default="full,full2,ap_site,ap_only,flat")
    a = ap.parse_args()
    example = a.example or os.path.join(a.lib, "lqos.example")
    root = Path(a.out or tempfile.mkdtemp(prefix="lqos-native-repro-"))
    root.mkdir(parents=True, exist_ok=True)
    os.chmod(root, 0o700)

    results = []
    cases = [(s, m) for m in a.modes.split(",") for s in ("shaped_devices", "libreqos_network")]
    for source, mode in cases:
        r = run_case(root, a.lib, example, source, mode)
        if r["ok"]:
            r["policy"] = node_policy(r["case"])
            r["policy_kept"] = policy_kept(r["policy"])
            r["circuit"] = circuit_plan(r["case"])
        r["case"] = str(r["case"])
        results.append(r)

    summary = {"lib": a.lib, "scratch": str(root), "results": results}
    (root / "results.json").write_text(json.dumps(summary, indent=2))

    print(f"scratch: {root}")
    print(f"{'case':<26} {'compile':<8} {'kept':<6} node rates (compiled compatibility tree, down/up Mbps)")
    for r in results:
        name = f'{r["source"]}/{r["mode"]}'
        if not r["ok"]:
            print(f"{name:<26} {'FAIL':<8} {'-':<6} {r['stderr'].splitlines()[-1] if r['stderr'] else ''}")
            continue
        rates = ", ".join(
            f'{k}={v["compiled_compatibility"]["down"]}/{v["compiled_compatibility"]["up"]}'
            if v["compiled_compatibility"] else f"{k}=absent"
            for k, v in r["policy"].items())
        print(f"{name:<26} {'ok':<8} {str(r['policy_kept']):<6} {rates}")
    leaks = [w for r in results for w in r["outside_writes"]]
    if leaks:
        print("ERROR: writes outside the scratch folder:\n  " + "\n  ".join(leaks))
        return 2

    # The control for each mode is the pair source in the same mode: same nodes, rates and circuit.
    # Only full and full2 keep the input tree. The pair path ignores the mode (source "legacy/manual"),
    # and ap_site, ap_only and flat project the native tree by design, so they are not compared.
    defects = []
    for mode in (m for m in a.modes.split(",") if m in ("full", "full2")):
        pair, native = (next(r for r in results if r["source"] == s and r["mode"] == mode)
                        for s in ("shaped_devices", "libreqos_network"))
        if not (pair["ok"] and native["ok"]):
            continue
        same_nodes = all(pair["policy"][k]["compiled_compatibility"] == native["policy"][k]["compiled_compatibility"]
                         for k in pair["policy"])
        if not same_nodes or pair["circuit"] != native["circuit"]:
            defects.append(mode)
    summary["defect_modes"] = defects
    (root / "results.json").write_text(json.dumps(summary, indent=2))
    others = [m for m in a.modes.split(",") if m not in ("full", "full2")]
    if others:
        print("not compared (projection by design): " + ", ".join(others))
    if defects:
        print("DEFECT: libreqos_network gives other node policy than shaped_devices in: " + ", ".join(defects))
        return 1
    print("No defect: libreqos_network and shaped_devices give the same node policy in each mode.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
