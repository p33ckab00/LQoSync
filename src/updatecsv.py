#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
updatecsv.py - MikroTik to LibreQoS sync with Telegram notification, preflight,
config-driven defaults, computation report, and flexible network modes.

Network modes:
  grouped_router_root  - router is root; generated PPP/DHCP/HS/plan nodes are children
  logical_core_root    - one root with LCORE buckets; clients are balanced across detected CPU cores minus reserved cores
  flat_router_root     - router is root; clients parent directly to router root
  flat_global_root     - one global root; clients from all routers parent directly to it
  no_parent            - CSV Parent Node is blank; network.json is minimally touched
"""

import csv
import hashlib
import html
import io
import json
import logging
import os
import random
import re
import string
import subprocess
import time
import urllib.parse
import urllib.request
from copy import deepcopy
from typing import Any, Dict, List, Optional, Tuple

import routeros_api

CONFIG_JSON = "config.json"

FIELDNAMES = [
    "Circuit ID",
    "Circuit Name",
    "Device ID",
    "Device Name",
    "Parent Node",
    "MAC",
    "IPv4",
    "IPv6",
    "Download Min Mbps",
    "Upload Min Mbps",
    "Download Max Mbps",
    "Upload Max Mbps",
    "Comment",
]

DEFAULT_CONFIG: Dict[str, Any] = {
    "runtime": {
        "scan_interval_active": 25,
        "scan_interval_idle": 120,
        "error_retry_interval": 30,
        "id_length": 8,
        "log_file": "",
        "remove_inactive_entries": True,
    },
    "libreqos": {
        "enabled": True,
        "command": "/opt/libreqos/src/LibreQoS.py",
        "timeout_seconds": 300,
        "sudo": True,
        "shaped_devices_csv": "ShapedDevices.csv",
        "network_json": "network.json",
    },
    "network": {
        "mode": "grouped_router_root",
        "preserve_network_config": False,
        "root_name": "",
        "root_type": "site",
        "root_download_mbps": 440,
        "root_upload_mbps": 440,
        "logical_roots": {
            "enabled": False,
            "prefix": "LCORE",
            "count_from_cpu": True,
            "reserve_cores": 2,
            "fallback_count": 4,
            "min_count": 1,
            "max_count": 32,
            "distribution": "sticky_least_loaded",
            "rebalance": True,
            "rebalance_threshold_clients": 3,
            "max_rebalance_moves_per_cycle": 1,
            "state_file": "logical_core_assignments.json",
            "capacity_mode": "full_root"
        },
    },
    "bandwidth_defaults": {
        "min_rate_percentage": 0.50,
        "safe_min_mbps": 0.128,
        "pppoe_default_profile": "default",
        "pppoe_default_rate": "10M/10M",
        "pppoe_aggregate_factor": 0.30,
        "dhcp_default_per_client_mbps": 15,
        "dhcp_download_factor": 0.5,
        "dhcp_upload_factor": 0.5,
        "hotspot_default_per_client_mbps": 10,
        "hotspot_download_factor": 1.0,
        "hotspot_upload_factor": 1.0,
    },
    "telegram": {
        "enabled": False,
        "bot_token_env": "LQOSYNC_TELEGRAM_BOT_TOKEN",
        "chat_id_env": "LQOSYNC_TELEGRAM_CHAT_ID",
        "bot_token": "",
        "chat_id": "",
        "parse_mode": "HTML",
        "notify_on_start": True,
        "notify_on_changes": True,
        "notify_on_no_changes": False,
        "notify_on_error": True,
        "notify_on_preflight_warning": True,
        "include_added": True,
        "include_updated": True,
        "include_removed": True,
        "include_computation": True,
        "include_telemetry": True,
        "max_list_items": 15,
    },
    "routers": [],
}

logger = logging.getLogger("lqosync")

# =========================
# REGEX
# =========================
RE_PIPE = re.compile(r"\|(\d+(?:\.\d+)?)M/(\d+(?:\.\d+)?)M", re.IGNORECASE)
RE_PAIR = re.compile(r"(\d+(?:\.\d+)?)M/(\d+(?:\.\d+)?)M", re.IGNORECASE)
RE_SINGLE = re.compile(r"(\d+(?:\.\d+)?)M", re.IGNORECASE)
RE_UNIT = re.compile(r"^(\d+(?:\.\d+)?)([kKmMgG]?)$")
RE_PLAN_NAME = re.compile(r"^([^|]+)\|")


# =========================
# GENERIC HELPERS
# =========================
def deep_merge(default: Dict[str, Any], override: Dict[str, Any]) -> Dict[str, Any]:
    result = deepcopy(default)
    for key, value in (override or {}).items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key] = deep_merge(result[key], value)
        else:
            result[key] = value
    return result


def read_json_file(filename: str, default=None):
    if default is None:
        default = {}
    try:
        if not os.path.exists(filename):
            return default
        with open(filename, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as e:
        logger.error("Failed reading %s: %s", filename, e)
        return default


def load_config() -> Dict[str, Any]:
    raw = read_json_file(CONFIG_JSON, {})

    # Backward compatibility for old root-level keys.
    if "preserve_network_config" in raw:
        raw.setdefault("network", {})["preserve_network_config"] = raw.get("preserve_network_config")
    if "flat_network" in raw and "network" not in raw:
        raw["network"] = {}
        raw["network"]["mode"] = "flat_router_root" if raw.get("flat_network") else "grouped_router_root"
    if "no_parent" in raw and raw.get("no_parent"):
        raw.setdefault("network", {})["mode"] = "no_parent"

    return deep_merge(DEFAULT_CONFIG, raw)


def setup_logging(config: Dict[str, Any]) -> None:
    handlers: List[logging.Handler] = [logging.StreamHandler()]
    log_file = str(config.get("runtime", {}).get("log_file", "")).strip()
    if log_file:
        handlers.append(logging.FileHandler(log_file, encoding="utf-8"))

    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s - %(levelname)s - %(message)s",
        handlers=handlers,
        force=True,
    )


def elapsed(start: float) -> str:
    secs = time.time() - start
    if secs < 60:
        return f"{secs:.2f}s"
    mins, secs = divmod(secs, 60)
    return f"{int(mins)}m {secs:.2f}s"


def generate_short_id(config: Dict[str, Any]) -> str:
    length = int(config.get("runtime", {}).get("id_length", 8))
    return "".join(random.choices(string.ascii_uppercase + string.digits, k=length))


def read_file_text(filename: str, default: str = "") -> str:
    try:
        if not os.path.exists(filename):
            return default
        with open(filename, "r", encoding="utf-8", newline="") as f:
            return f.read()
    except Exception as e:
        logger.error("Failed reading raw file %s: %s", filename, e)
        return default


def write_text_file_if_changed(filename: str, new_content: str) -> bool:
    current_content = read_file_text(filename, default="")
    if current_content == new_content:
        logger.info("No on-disk changes for %s", filename)
        return False
    with open(filename, "w", encoding="utf-8", newline="") as f:
        f.write(new_content)
    logger.info("Wrote updated file %s", filename)
    return True


def render_network_json(data: Dict[str, Any]) -> str:
    return json.dumps(data, indent=4, sort_keys=True, ensure_ascii=False) + "\n"


def write_json_file_if_changed(filename: str, data: Dict[str, Any]) -> bool:
    return write_text_file_if_changed(filename, render_network_json(data))


def safe_float(value: Any, default: float = 0.0) -> float:
    try:
        return float(value)
    except Exception:
        return default


# =========================
# REPORTING
# =========================
def new_report() -> Dict[str, Any]:
    return {
        "started_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "preflight": [],
        "routers": {},
        "added": [],
        "updated": [],
        "removed": [],
        "duplicates": [],
        "errors": [],
        "computations": [],
        "node_updates": [],
        "telemetry": {},
        "libreqos": None,
    }


def report_status(report: Dict[str, Any], name: str, ok: bool, detail: str = "") -> None:
    report["preflight"].append({"name": name, "ok": ok, "detail": detail})


def report_error(report: Dict[str, Any], where: str, error: Any) -> None:
    msg = str(error)
    report["errors"].append({"where": where, "error": msg})
    logger.error("%s: %s", where, msg)


def record_router_stat(report: Dict[str, Any], router_name: str, key: str, value: Any) -> None:
    report["routers"].setdefault(router_name, {})[key] = value


def record_added(report: Dict[str, Any], kind: str, code: str, ip: str, mac: str, parent: str, down: Any, up: Any) -> None:
    report["added"].append({
        "type": kind,
        "name": code,
        "ip": ip,
        "mac": mac,
        "parent": parent,
        "down": down,
        "up": up,
    })


def record_updated(report: Dict[str, Any], kind: str, code: str, ip: str, parent: str, changes: Dict[str, Tuple[str, str]]) -> None:
    report["updated"].append({
        "type": kind,
        "name": code,
        "ip": ip,
        "parent": parent,
        "changes": changes,
    })


def record_removed(report: Dict[str, Any], code: str, ip: str, parent: str, comment: str) -> None:
    report["removed"].append({"name": code, "ip": ip, "parent": parent, "comment": comment})


def record_computation(report: Dict[str, Any], **kwargs) -> None:
    report["computations"].append(kwargs)


def record_node_update(report: Dict[str, Any], node: str, old: Any, new: Any) -> None:
    report["node_updates"].append({"node": node, "old": old, "new": new})


# =========================
# CSV / LibreQoS FILES
# =========================
def get_shaped_devices_csv(config: Dict[str, Any]) -> str:
    return str(config.get("libreqos", {}).get("shaped_devices_csv", "ShapedDevices.csv"))


def get_network_json(config: Dict[str, Any]) -> str:
    return str(config.get("libreqos", {}).get("network_json", "network.json"))


def read_shaped_devices_csv(config: Dict[str, Any]) -> Dict[str, dict]:
    filename = get_shaped_devices_csv(config)
    if not os.path.exists(filename):
        return {}
    try:
        with open(filename, "r", newline="", encoding="utf-8") as f:
            reader = csv.DictReader(f)
            data = {}
            for row in reader:
                circuit_name = row.get("Circuit Name", "").strip()
                if circuit_name:
                    data[circuit_name] = row
            logger.info("Loaded %d existing shaped devices.", len(data))
            return data
    except Exception as e:
        logger.error("Failed reading %s: %s", filename, e)
        return {}


def render_shaped_devices_csv(data: Dict[str, dict]) -> str:
    output = io.StringIO(newline="")
    writer = csv.DictWriter(output, fieldnames=FIELDNAMES, lineterminator="\n")
    writer.writeheader()
    rows = []
    for row in data.values():
        rows.append({field: str(row.get(field, "")) for field in FIELDNAMES})
    rows.sort(key=lambda r: (
        r.get("Parent Node", ""),
        r.get("Circuit Name", ""),
        r.get("Device Name", ""),
        r.get("IPv4", ""),
        r.get("MAC", ""),
    ))
    writer.writerows(rows)
    return output.getvalue()


def write_shaped_devices_csv_if_changed(config: Dict[str, Any], data: Dict[str, dict]) -> bool:
    filename = get_shaped_devices_csv(config)
    changed = write_text_file_if_changed(filename, render_shaped_devices_csv(data))
    if changed:
        logger.info("Wrote %d shaped devices.", len(data))
    else:
        logger.info("ShapedDevices.csv unchanged (%d shaped devices).", len(data))
    return changed


# =========================
# TELEGRAM
# =========================
def telegram_enabled(config: Dict[str, Any]) -> bool:
    return bool(config.get("telegram", {}).get("enabled", False))


def telegram_credentials(config: Dict[str, Any]) -> Tuple[str, str]:
    tg = config.get("telegram", {})
    token = os.environ.get(str(tg.get("bot_token_env", ""))) or str(tg.get("bot_token", ""))
    chat_id = os.environ.get(str(tg.get("chat_id_env", ""))) or str(tg.get("chat_id", ""))
    return token.strip(), chat_id.strip()


def telegram_send(config: Dict[str, Any], message: str) -> bool:
    if not telegram_enabled(config):
        return False

    token, chat_id = telegram_credentials(config)
    if not token or not chat_id:
        logger.warning("Telegram enabled but bot token or chat id is missing.")
        return False

    tg = config.get("telegram", {})
    url = f"https://api.telegram.org/bot{token}/sendMessage"

    # Telegram limit is 4096 chars; keep margin for safety.
    chunks = []
    text = message
    while len(text) > 3900:
        cut = text.rfind("\n", 0, 3900)
        if cut <= 0:
            cut = 3900
        chunks.append(text[:cut])
        text = text[cut:].lstrip()
    chunks.append(text)

    ok_all = True
    for chunk in chunks:
        payload = urllib.parse.urlencode({
            "chat_id": chat_id,
            "text": chunk,
            "parse_mode": tg.get("parse_mode", "HTML"),
            "disable_web_page_preview": "true",
        }).encode("utf-8")
        try:
            req = urllib.request.Request(url, data=payload, method="POST")
            with urllib.request.urlopen(req, timeout=10) as resp:
                ok_all = ok_all and (200 <= resp.status < 300)
        except Exception as e:
            ok_all = False
            logger.error("Telegram send failed: %s", e)
    return ok_all


def esc(value: Any) -> str:
    return html.escape(str(value if value is not None else ""))


def compact_list(items: List[dict], formatter, max_items: int) -> List[str]:
    lines = []
    for item in items[:max_items]:
        lines.append(formatter(item))
    if len(items) > max_items:
        lines.append(f"...and {len(items) - max_items} more")
    return lines


def build_telegram_message(config: Dict[str, Any], report: Dict[str, Any], files_changed: bool, any_updates: bool, next_sleep: int) -> str:
    tg = config.get("telegram", {})
    max_items = int(tg.get("max_list_items", 15))
    network_mode = config.get("network", {}).get("mode", "grouped_router_root")

    error_count = len(report.get("errors", []))
    warn_count = len([p for p in report.get("preflight", []) if not p.get("ok")])
    lib = report.get("libreqos") or {}
    lib_status = lib.get("status", "skipped")

    if error_count:
        title = "ALERT: <b>LQoSync completed with errors</b>"
    elif files_changed:
        title = "OK: <b>LQoSync update applied</b>"
    elif any_updates:
        title = "INFO: <b>LQoSync checked: in-memory changes only</b>"
    else:
        title = "OK: <b>LQoSync checked: no changes</b>"

    lines = [title, ""]
    lines.append(f"<b>Mode:</b> {esc(network_mode)}")
    lines.append(f"<b>Duration:</b> {esc(report.get('telemetry', {}).get('cycle_duration', 'n/a'))}")
    lines.append(f"<b>Files changed:</b> {'Yes' if files_changed else 'No'}")
    lines.append(f"<b>LibreQoS:</b> {esc(lib_status)}")
    lines.append(f"<b>Next scan:</b> {next_sleep}s")
    lines.append("")

    # Router summary
    if report.get("routers"):
        lines.append("<b>Routers</b>")
        for rname, stats in report["routers"].items():
            parts = []
            for key in ("pppoe_active", "dhcp_active", "hotspot_active", "active_total"):
                if key in stats:
                    parts.append(f"{key.replace('_', ' ')}: {stats[key]}")
            if "duration" in stats:
                parts.append(f"time: {stats['duration']}")
            lines.append(f"- {esc(rname)}  {esc(', '.join(parts))}")
        lines.append("")

    lines.append("<b>Summary</b>")
    lines.append(f"- Added: {len(report.get('added', []))}")
    lines.append(f"- Updated: {len(report.get('updated', []))}")
    lines.append(f"- Removed: {len(report.get('removed', []))}")
    lines.append(f"- Node updates: {len(report.get('node_updates', []))}")
    lines.append(f"- Preflight warnings: {warn_count}")
    lines.append(f"- Errors: {error_count}")
    lines.append("")

    if tg.get("include_added", True) and report.get("added"):
        lines.append("<b>Added</b>")
        lines += compact_list(
            report["added"],
            lambda x: f"- {esc(x['type'])}: {esc(x['name'])}  {esc(x.get('ip',''))}  {esc(x.get('down',''))}/{esc(x.get('up',''))} Mbps -> {esc(x.get('parent',''))}",
            max_items,
        )
        lines.append("")

    if tg.get("include_updated", True) and report.get("updated"):
        lines.append("<b>Updated</b>")
        def fmt_update(x):
            change_keys = ", ".join(list(x.get("changes", {}).keys())[:5])
            return f"- {esc(x['type'])}: {esc(x['name'])}  {esc(x.get('ip',''))}  {esc(change_keys)}"
        lines += compact_list(report["updated"], fmt_update, max_items)
        lines.append("")

    if tg.get("include_removed", True) and report.get("removed"):
        lines.append("<b>Removed / inactive</b>")
        lines += compact_list(
            report["removed"],
            lambda x: f"- {esc(x['name'])}  {esc(x.get('ip',''))}  {esc(x.get('comment',''))}",
            max_items,
        )
        lines.append("")

    if report.get("duplicates"):
        lines.append("<b>Duplicate IP warnings</b>")
        lines += compact_list(
            report["duplicates"],
            lambda x: f"- {esc(x.get('ip'))}: {esc(x.get('existing'))} vs {esc(x.get('new'))}",
            max_items,
        )
        lines.append("")

    if tg.get("include_computation", True) and report.get("computations"):
        lines.append("<b>Computation</b>")
        def fmt_comp(c):
            if c.get("mode") == "flat_ppp":
                return (
                    f"- {esc(c.get('node'))}: sum {esc(c.get('sum_down'))}/{esc(c.get('sum_up'))} Mbps  "
                    f"{esc(c.get('factor'))} = {esc(c.get('final_down'))}/{esc(c.get('final_up'))} Mbps "
                    f"cap {esc(c.get('cap_down'))}/{esc(c.get('cap_up'))}"
                )
            if c.get("mode") == "flat_clients":
                return f"- {esc(c.get('node'))}: {esc(c.get('users'))} clients direct to root"
            return (
                f"- {esc(c.get('node'))}: {esc(c.get('users'))} users, "
                f"sum {esc(c.get('sum_down'))}/{esc(c.get('sum_up'))} Mbps  "
                f"{esc(c.get('factor_down'))}/{esc(c.get('factor_up'))} = "
                f"{esc(c.get('final_down'))}/{esc(c.get('final_up'))} Mbps"
            )
        lines += compact_list(report["computations"], fmt_comp, max_items)
        lines.append("")

    if report.get("preflight") and (warn_count or tg.get("notify_on_preflight_warning", True)):
        lines.append("<b>Preflight</b>")
        for p in report["preflight"][:max_items]:
            icon = "OK" if p.get("ok") else "WARN"
            detail = f"  {esc(p.get('detail'))}" if p.get("detail") else ""
            lines.append(f"{icon} {esc(p.get('name'))}{detail}")
        if len(report["preflight"]) > max_items:
            lines.append(f"...and {len(report['preflight']) - max_items} more")
        lines.append("")

    if report.get("errors"):
        lines.append("<b>Errors</b>")
        lines += compact_list(
            report["errors"],
            lambda x: f"- {esc(x.get('where'))}: {esc(x.get('error'))}",
            max_items,
        )
        lines.append("")

    if tg.get("include_telemetry", True):
        tel = report.get("telemetry", {})
        lines.append("<b>Telemetry</b>")
        for key in ("csv_changed", "network_json_changed", "cycle_duration"):
            if key in tel:
                lines.append(f"- {esc(key)}: {esc(tel[key])}")

    return "\n".join(lines).strip()


# =========================
# PREFLIGHT
# =========================
def path_writable(path: str) -> bool:
    target = path if os.path.isdir(path) else os.path.dirname(os.path.abspath(path)) or "."
    return os.access(target, os.W_OK)


def run_preflight(config: Dict[str, Any], report: Dict[str, Any]) -> bool:
    ok_all = True

    routers = config.get("routers", [])
    ok = isinstance(routers, list) and len(routers) > 0
    report_status(report, "routers configured", ok, f"count={len(routers) if isinstance(routers, list) else 0}")
    ok_all = ok_all and ok

    shaped = get_shaped_devices_csv(config)
    netjson = get_network_json(config)

    ok = path_writable(shaped)
    report_status(report, "ShapedDevices.csv writable", ok, shaped)
    ok_all = ok_all and ok

    ok = path_writable(netjson)
    report_status(report, "network.json writable", ok, netjson)
    ok_all = ok_all and ok

    libcfg = config.get("libreqos", {})
    cmd = str(libcfg.get("command", ""))
    if libcfg.get("enabled", True):
        ok = bool(cmd and os.path.exists(cmd))
        report_status(report, "LibreQoS command found", ok, cmd)
        ok_all = ok_all and ok
    else:
        report_status(report, "LibreQoS command", True, "disabled")

    mode = str(config.get("network", {}).get("mode", "grouped_router_root"))
    ok = mode in {"grouped_router_root", "logical_core_root", "flat_router_root", "flat_global_root", "no_parent"}
    report_status(report, "network mode valid", ok, mode)
    ok_all = ok_all and ok

    if mode == "logical_core_root":
        count = logical_root_count(config)
        cpu_count = os.cpu_count() or 0
        reserve = int(config.get("network", {}).get("logical_roots", {}).get("reserve_cores", 0))
        ok = count >= 1
        report_status(report, "logical core buckets", ok, f"cpu={cpu_count}, reserve={reserve}, buckets={count}")
        ok_all = ok_all and ok

    if telegram_enabled(config):
        token, chat_id = telegram_credentials(config)
        ok = bool(token and chat_id)
        report_status(report, "Telegram credentials", ok, "env/config present" if ok else "missing token/chat_id")
        ok_all = ok_all and ok
    else:
        report_status(report, "Telegram", True, "disabled")

    return ok_all


# =========================
# MIKROTIK HELPERS
# =========================
def connect_to_router(router: dict, report: Dict[str, Any], retries: int = 3):
    for attempt in range(1, retries + 1):
        try:
            pool = routeros_api.RouterOsApiPool(
                router["address"],
                username=router["username"],
                password=router["password"],
                port=int(router.get("port", 8728)),
                plaintext_login=True,
            )
            api = pool.get_api()
            logger.info("Connected to %s (%s)", router.get("name"), router.get("address"))
            return pool, api
        except Exception as e:
            logger.warning("Connection failed to %s attempt %d/%d: %s", router.get("name", "unknown"), attempt, retries, e)
            if attempt < retries:
                time.sleep(3)
    report_error(report, f"connect:{router.get('name', 'unknown')}", "failed after retries")
    return None, None


def get_resource_data(api, resource_path: str, **filters) -> list:
    try:
        resource = api.get_resource(resource_path)
        result = resource.get(**filters) if filters else resource.get()
        if isinstance(result, list):
            return result
        try:
            return list(result)
        except Exception:
            return []
    except Exception as e:
        logger.error("Failed reading MikroTik resource %s: %s", resource_path, e)
        return []


# =========================
# BANDWIDTH HELPERS
# =========================
def convert_to_mbps(value_str: str) -> float:
    if not value_str:
        return 0.0
    value_str = str(value_str).strip()
    match = RE_UNIT.match(value_str)
    if not match:
        return 0.0
    number = float(match.group(1))
    unit = match.group(2).lower()
    multiplier = {"": 1.0, "k": 0.001, "m": 1.0, "g": 1000.0}.get(unit, 1.0)
    return number * multiplier


def parse_rate_limit(rate_limit: str) -> Tuple[float, float]:
    if not rate_limit:
        return 0.0, 0.0
    try:
        primary = rate_limit.split()[0]
        rx_raw, tx_raw = primary.split("/")
        return convert_to_mbps(rx_raw), convert_to_mbps(tx_raw)
    except Exception:
        return 0.0, 0.0


def parse_comment_bandwidth(comment: str) -> Optional[Tuple[float, float]]:
    if not comment:
        return None
    comment = str(comment).strip()
    m = RE_PIPE.search(comment)
    if m:
        return float(m.group(1)), float(m.group(2))
    m = RE_PAIR.search(comment)
    if m:
        return float(m.group(1)), float(m.group(2))
    m = RE_SINGLE.search(comment)
    if m:
        speed = float(m.group(1))
        return speed, speed
    return None


def parse_speed_from_profile_name(profile_name: str) -> Optional[Tuple[float, float]]:
    if not profile_name:
        return None
    m = re.search(r"(\d+(?:\.\d+)?)([kKmMgG]?)\s*$", profile_name)
    if not m:
        return None
    number = float(m.group(1))
    unit = m.group(2).lower()
    mbps = {"": 1.0, "k": 0.001, "m": 1.0, "g": 1000.0}.get(unit, 1.0) * number
    if mbps <= 0:
        return None
    return mbps, mbps


def safe_int_mbps(config: Dict[str, Any], value: float) -> float:
    minimum = float(config.get("bandwidth_defaults", {}).get("safe_min_mbps", 0.128))
    return max(round(float(value), 3), minimum)


def calculate_user_min_max(config: Dict[str, Any], base_rx: float, base_tx: float) -> Tuple[str, str, str, str]:
    bwd = config.get("bandwidth_defaults", {})
    min_pct = float(bwd.get("min_rate_percentage", 0.50))
    rx_max = safe_int_mbps(config, base_rx)
    tx_max = safe_int_mbps(config, base_tx)
    rx_min = safe_int_mbps(config, base_rx * min_pct)
    tx_min = safe_int_mbps(config, base_tx * min_pct)
    return str(rx_min), str(tx_min), str(rx_max), str(tx_max)


def get_plan_factors(profile_name: str) -> Tuple[float, float]:
    # Reserved for future per-plan factor rules. Currently neutral.
    return 1.0, 1.0


# =========================
# NETWORK MODE HELPERS
# =========================
def network_mode(config: Dict[str, Any]) -> str:
    return str(config.get("network", {}).get("mode", "grouped_router_root")).strip()


def root_node_name(config: Dict[str, Any], router_name: str) -> str:
    mode = network_mode(config)
    net = config.get("network", {})
    if mode in ("flat_global_root", "logical_core_root"):
        return str(net.get("root_name") or "LibreQoS-ROOT")
    if mode in ("grouped_router_root", "flat_router_root"):
        return router_name
    return ""


def sanitize_node_part(value: Any) -> str:
    text = str(value or "").strip()
    text = re.sub(r"[^A-Za-z0-9_.:-]+", "-", text)
    text = re.sub(r"-{2,}", "-", text).strip("-")
    return text or "NODE"


def logical_roots_config(config: Dict[str, Any]) -> Dict[str, Any]:
    return config.get("network", {}).get("logical_roots", {}) or {}


def logical_root_count(config: Dict[str, Any]) -> int:
    lr = logical_roots_config(config)

    fallback = int(lr.get("fallback_count", 4))
    min_count = max(1, int(lr.get("min_count", 1)))
    max_count = max(min_count, int(lr.get("max_count", 32)))

    if bool(lr.get("count_from_cpu", True)):
        cpu_count = os.cpu_count() or fallback
        reserve = int(lr.get("reserve_cores", 2))
        count = cpu_count - reserve
    else:
        count = int(lr.get("count", fallback))

    count = max(min_count, count)
    count = min(max_count, count)
    return count


def logical_bucket_names(config: Dict[str, Any]) -> List[str]:
    lr = logical_roots_config(config)
    prefix = sanitize_node_part(lr.get("prefix", "LCORE"))
    count = logical_root_count(config)
    width = max(2, len(str(count)))
    return [f"{prefix}-{i:0{width}d}" for i in range(1, count + 1)]


def logical_capacity_for_bucket(config: Dict[str, Any], count: int) -> Tuple[float, float]:
    net = config.get("network", {})
    lr = logical_roots_config(config)
    root_down = float(net.get("root_download_mbps", 440))
    root_up = float(net.get("root_upload_mbps", 440))
    capacity_mode = str(lr.get("capacity_mode", "full_root")).strip().lower()

    if capacity_mode == "equal_share":
        return safe_int_mbps(config, root_down / max(1, count)), safe_int_mbps(config, root_up / max(1, count))

    # Sa logical-core grouping, safer default ito:
    # the global root caps total bandwidth, while each logical bucket does not hard-cap bursts.
    return root_down, root_up


def ensure_root_node(config: Dict[str, Any], network_config: Dict[str, Any], router_name: str) -> Optional[str]:
    mode = network_mode(config)
    if mode == "no_parent":
        return None

    net = config.get("network", {})
    root = root_node_name(config, router_name)
    if not root:
        return None

    root_down = float(net.get("root_download_mbps", 440))
    root_up = float(net.get("root_upload_mbps", 440))
    root_type = str(net.get("root_type", "site"))

    desired = {
        "downloadBandwidthMbps": root_down,
        "uploadBandwidthMbps": root_up,
        "type": root_type,
        "children": {},
    }

    if root not in network_config or not isinstance(network_config.get(root), dict):
        network_config[root] = desired
    else:
        network_config[root]["downloadBandwidthMbps"] = root_down
        network_config[root]["uploadBandwidthMbps"] = root_up
        network_config[root]["type"] = root_type
        network_config[root].setdefault("children", {})
        if not isinstance(network_config[root]["children"], dict):
            network_config[root]["children"] = {}

    if mode == "logical_core_root":
        ensure_logical_core_children(config, network_config, root)

    return root


def ensure_logical_core_children(config: Dict[str, Any], network_config: Dict[str, Any], root: str) -> Dict[str, Any]:
    network_config[root].setdefault("children", {})
    children = network_config[root]["children"]
    if not isinstance(children, dict):
        children = {}
        network_config[root]["children"] = children

    names = logical_bucket_names(config)
    bucket_down, bucket_up = logical_capacity_for_bucket(config, len(names))

    for name in names:
        desired = {
            "downloadBandwidthMbps": bucket_down,
            "uploadBandwidthMbps": bucket_up,
            "type": "site",
            "children": {},
        }
        if name not in children or not isinstance(children.get(name), dict):
            children[name] = desired
        else:
            children[name]["downloadBandwidthMbps"] = bucket_down
            children[name]["uploadBandwidthMbps"] = bucket_up
            children[name]["type"] = "site"
            children[name].setdefault("children", {})
            if not isinstance(children[name]["children"], dict):
                children[name]["children"] = {}

    # Remove old logical buckets for same prefix when CPU/reserved count changes.
    prefix = sanitize_node_part(logical_roots_config(config).get("prefix", "LCORE"))
    wanted = set(names)
    for old in list(children.keys()):
        if re.match(rf"^{re.escape(prefix)}-\d+$", old) and old not in wanted:
            del children[old]

    return children


def parent_for_client(config: Dict[str, Any], router_name: str, grouped_parent: str) -> str:
    mode = network_mode(config)
    if mode == "no_parent":
        return ""
    if mode in ("flat_router_root", "flat_global_root"):
        return root_node_name(config, router_name)
    # grouped_router_root and logical_core_root both use the computed grouped/logical parent.
    return grouped_parent


def network_children_for_grouped(config: Dict[str, Any], network_config: Dict[str, Any], router_name: str) -> Optional[Dict[str, Any]]:
    if network_mode(config) != "grouped_router_root":
        return None
    root = ensure_root_node(config, network_config, router_name)
    if not root:
        return None
    return network_config[root].setdefault("children", {})


def update_network_json_skeleton(config: Dict[str, Any], routers: List[dict]) -> Dict[str, Any]:
    netfile = get_network_json(config)
    preserve = bool(config.get("network", {}).get("preserve_network_config", False))
    network_config = read_json_file(netfile, {}) if preserve else {}
    for router in routers:
        ensure_root_node(config, network_config, router.get("name", "unknown"))
    return network_config


def set_child_node(report: Dict[str, Any], children: Dict[str, Any], node_name: str, new_node: Dict[str, Any]) -> bool:
    old = children.get(node_name)
    if old != new_node:
        children[node_name] = new_node
        record_node_update(report, node_name, old, new_node)
        return True
    return False


# =========================
# LOGICAL CORE ASSIGNMENT
# =========================
def logical_state_file(config: Dict[str, Any]) -> str:
    return str(logical_roots_config(config).get("state_file", "logical_core_assignments.json"))


def load_logical_core_state(config: Dict[str, Any]) -> None:
    if network_mode(config) != "logical_core_root":
        return

    state_file = logical_state_file(config)
    raw = read_json_file(state_file, {})
    assignments = raw.get("assignments", {}) if isinstance(raw, dict) else {}
    if not isinstance(assignments, dict):
        assignments = {}

    names = set(logical_bucket_names(config))
    assignments = {str(k): str(v) for k, v in assignments.items() if str(v) in names}

    config["_logical_state"] = {"assignments": assignments}
    config["_logical_counts"] = {name: 0 for name in names}
    config["_logical_seen"] = set()
    config["_logical_state_changed"] = False


def write_logical_core_state_if_changed(config: Dict[str, Any]) -> bool:
    if network_mode(config) != "logical_core_root":
        return False

    state = config.get("_logical_state", {"assignments": {}})
    assignments = state.get("assignments", {})
    if not isinstance(assignments, dict):
        assignments = {}

    payload = {
        "generated_at": time.strftime("%Y-%m-%d %H:%M:%S"),
        "mode": "logical_core_root",
        "bucket_count": logical_root_count(config),
        "buckets": logical_bucket_names(config),
        "assignments": dict(sorted(assignments.items())),
    }

    changed = write_json_file_if_changed(logical_state_file(config), payload)
    if changed:
        config["_logical_state_changed"] = False
    return changed


def least_loaded_logical_bucket(config: Dict[str, Any]) -> str:
    names = logical_bucket_names(config)
    counts = config.setdefault("_logical_counts", {name: 0 for name in names})
    for name in names:
        counts.setdefault(name, 0)
    return min(names, key=lambda n: (int(counts.get(n, 0)), n))


def stable_hash_logical_bucket(config: Dict[str, Any], key: str) -> str:
    names = logical_bucket_names(config)
    digest = hashlib.sha256(str(key).encode("utf-8")).hexdigest()
    return names[int(digest, 16) % len(names)]


def logical_parent_for_client(config: Dict[str, Any], code: str, ipv4: str = "", mac: str = "") -> str:
    """
    Assign one client to one logical bucket.

    sticky_least_loaded:
      - keep existing assignment while client remains active
      - new clients go to least-loaded bucket
      - optional rebalance later moves only a small number per cycle

    stable_hash:
      - deterministic by code/mac/ip
      - no active balancing, but very stable
    """
    names = logical_bucket_names(config)
    if not names:
        return ""

    state = config.setdefault("_logical_state", {"assignments": {}})
    assignments = state.setdefault("assignments", {})
    counts = config.setdefault("_logical_counts", {name: 0 for name in names})
    seen = config.setdefault("_logical_seen", set())

    key = str(code or mac or ipv4).strip()
    if not key:
        key = generate_short_id(config)

    assigned = str(assignments.get(key, ""))
    distribution = str(logical_roots_config(config).get("distribution", "sticky_least_loaded")).strip().lower()

    if assigned not in names:
        if distribution == "stable_hash":
            assigned = stable_hash_logical_bucket(config, key)
        else:
            assigned = least_loaded_logical_bucket(config)
        assignments[key] = assigned
        config["_logical_state_changed"] = True

    seen.add(key)
    counts[assigned] = int(counts.get(assigned, 0)) + 1
    return assigned


def rebalance_logical_assignments(config: Dict[str, Any], existing_data: dict, active_codes: set, report: Dict[str, Any]) -> bool:
    if network_mode(config) != "logical_core_root":
        return False

    lr = logical_roots_config(config)
    if str(lr.get("distribution", "sticky_least_loaded")).strip().lower() == "stable_hash":
        return False

    names = logical_bucket_names(config)
    if not names:
        return False

    state = config.setdefault("_logical_state", {"assignments": {}})
    assignments = state.setdefault("assignments", {})
    updated = False

    active_codes = set(str(c) for c in active_codes)

    # Remove inactive clients from assignment state.
    for code in list(assignments.keys()):
        if code not in active_codes:
            del assignments[code]
            config["_logical_state_changed"] = True
            updated = True

    # Ensure active clients all have valid assignments.
    counts = {name: 0 for name in names}
    for code in sorted(active_codes):
        assigned = str(assignments.get(code, ""))
        if assigned not in names:
            assigned = min(names, key=lambda n: (counts[n], n))
            assignments[code] = assigned
            config["_logical_state_changed"] = True
            updated = True
        counts[assigned] += 1

    if not bool(lr.get("rebalance", True)):
        return updated

    threshold = max(1, int(lr.get("rebalance_threshold_clients", 3)))
    max_moves = max(0, int(lr.get("max_rebalance_moves_per_cycle", 1)))
    moves = 0

    while moves < max_moves:
        high = max(names, key=lambda n: (counts[n], n))
        low = min(names, key=lambda n: (counts[n], n))
        if counts[high] - counts[low] < threshold:
            break

        candidates = [code for code in sorted(active_codes) if assignments.get(code) == high]
        if not candidates:
            break

        code = candidates[-1]
        old_parent = high
        new_parent = low
        assignments[code] = new_parent
        counts[old_parent] -= 1
        counts[new_parent] += 1
        moves += 1
        updated = True
        config["_logical_state_changed"] = True

        if code in existing_data:
            old = str(existing_data[code].get("Parent Node", ""))
            if old != new_parent:
                existing_data[code]["Parent Node"] = new_parent
                kind = str(existing_data[code].get("Comment", "CLIENT") or "CLIENT")
                record_updated(
                    report,
                    kind,
                    code,
                    existing_data[code].get("IPv4", ""),
                    new_parent,
                    {"Parent Node": (old, new_parent)},
                )

    if moves:
        record_computation(
            report,
            mode="logical_core_rebalance",
            buckets=len(names),
            moves=moves,
            counts={name: counts[name] for name in names},
        )

    return updated

# =========================
# ENTRY HELPERS
# =========================
def create_new_entry(config: Dict[str, Any], code: str, device_name: str, mac: str = "", ipv4: str = "", comment: str = "") -> dict:
    return {
        "Circuit ID": generate_short_id(config),
        "Circuit Name": code,
        "Device ID": generate_short_id(config),
        "Device Name": device_name,
        "Parent Node": "",
        "MAC": mac,
        "IPv4": ipv4,
        "IPv6": "",
        "Download Min Mbps": "0",
        "Upload Min Mbps": "0",
        "Download Max Mbps": "0",
        "Upload Max Mbps": "0",
        "Comment": comment,
    }


def update_entry_values(entry: dict, new_values: dict) -> Dict[str, Tuple[str, str]]:
    changes: Dict[str, Tuple[str, str]] = {}
    for key, value in new_values.items():
        old = str(entry.get(key, ""))
        new = str(value)
        if old != new:
            entry[key] = new
            changes[key] = (old, new)
    return changes


def check_duplicate_ip(existing_data: dict, ip: str, code: str, report: Dict[str, Any]) -> bool:
    if not ip:
        return False
    for existing_code, row in existing_data.items():
        if existing_code == code:
            continue
        if row.get("IPv4", "").strip() == ip.strip():
            logger.warning("Duplicate IP %s between %s and %s", ip, existing_code, code)
            report["duplicates"].append({"ip": ip, "existing": existing_code, "new": code})
            return True
    return False


# =========================
# PPP PROFILE CACHE
# =========================
def preload_ppp_profiles(config: Dict[str, Any], api) -> dict:
    t0 = time.time()
    bwd = config.get("bandwidth_defaults", {})
    default_rate = str(bwd.get("pppoe_default_rate", "10M/10M"))
    profiles = get_resource_data(api, "/ppp/profile")
    cache = {}
    for p in profiles:
        name = p.get("name")
        comment = str(p.get("comment", "")).strip()
        rate = str(p.get("rate-limit", "")).strip()

        parsed = parse_comment_bandwidth(comment)
        if parsed:
            cache[name] = parsed
            continue

        rx, tx = parse_rate_limit(rate)
        if rx > 0 and tx > 0:
            cache[name] = (rx, tx)
            continue

        name_parsed = parse_speed_from_profile_name(name)
        if name_parsed:
            cache[name] = name_parsed
            logger.info("Profile '%s' speed from name: %.1f/%.1f Mbps", name, *name_parsed)
            continue

        cache[name] = parse_rate_limit(default_rate)
        logger.warning("Profile '%s' no speed info; using default %s", name, default_rate)

    logger.info("Preloaded %d PPP profiles. [%s]", len(cache), elapsed(t0))
    return cache


# =========================
# PPPoE
# =========================
def process_pppoe_users(config: Dict[str, Any], api, router: dict, existing_data: dict, network_config: dict, report: Dict[str, Any]) -> Tuple[set, bool]:
    if not router.get("pppoe", {}).get("enabled", False):
        return set(), False

    t0 = time.time()
    router_name = router["name"]
    per_plan_node = bool(router.get("pppoe", {}).get("per_plan_node", False))
    updated = False
    active_codes = set()
    bwd = config.get("bandwidth_defaults", {})
    mode = network_mode(config)

    ensure_root_node(config, network_config, router_name)
    children = network_children_for_grouped(config, network_config, router_name)

    if mode == "logical_core_root" and per_plan_node:
        logger.info("[PPPoE:%s] logical_core_root active; per_plan_node is ignored and clients parent directly to LCORE buckets.", router_name)

    t_fetch = time.time()
    secrets = {
        s["name"]: s
        for s in get_resource_data(api, "/ppp/secret")
        if s.get("name")
        and s.get("disabled", "false").lower() == "false"
        and s.get("inactive", "false").lower() == "false"
    }
    active = {a["name"]: a for a in get_resource_data(api, "/ppp/active") if a.get("name")}
    profile_cache = preload_ppp_profiles(config, api)
    logger.info("[PPPoE:%s] Fetched %d secrets, %d active. [fetch: %s]", router_name, len(secrets), len(active), elapsed(t_fetch))

    flat_ppp_total_rx = 0.0
    flat_ppp_total_tx = 0.0
    plan_totals: Dict[str, Dict[str, Any]] = {}
    default_profile = str(bwd.get("pppoe_default_profile", "default"))
    default_rate = str(bwd.get("pppoe_default_rate", "10M/10M"))

    common_users = set(active.keys()) & set(secrets.keys())

    for username in common_users:
        active_row = active[username]
        secret = secrets[username]

        ip_address = str(active_row.get("address", "")).strip()
        mac = str(active_row.get("caller-id") or secret.get("caller-id") or "").strip()
        mac_clean = mac.replace(":", "").upper() if mac else ""
        profile_name = str(secret.get("profile", default_profile)).strip() or default_profile
        code = username if username else mac_clean
        active_codes.add(code)

        secret_comment = str(secret.get("comment", "")).strip()
        active_comment = str(active_row.get("comment", "")).strip()
        parsed = parse_comment_bandwidth(secret_comment) or parse_comment_bandwidth(active_comment) or profile_cache.get(profile_name) or parse_rate_limit(default_rate)
        base_rx, base_tx = parsed
        rx_min, tx_min, rx_max, tx_max = calculate_user_min_max(config, base_rx, base_tx)

        if mode == "logical_core_root":
            grouped_parent = logical_parent_for_client(config, code, ip_address, mac)
            flat_ppp_total_rx += base_rx
            flat_ppp_total_tx += base_tx
        elif mode == "grouped_router_root":
            if per_plan_node:
                grouped_parent = f"{profile_name}-{router_name}"
                plan_totals.setdefault(grouped_parent, {"rx": 0.0, "tx": 0.0, "count": 0, "profile": profile_name})
                plan_totals[grouped_parent]["rx"] += base_rx
                plan_totals[grouped_parent]["tx"] += base_tx
                plan_totals[grouped_parent]["count"] += 1
            else:
                grouped_parent = f"PPP-{router_name}"
                flat_ppp_total_rx += base_rx
                flat_ppp_total_tx += base_tx
        else:
            grouped_parent = root_node_name(config, router_name)
            flat_ppp_total_rx += base_rx
            flat_ppp_total_tx += base_tx

        parent_node = parent_for_client(config, router_name, grouped_parent)

        is_new = code not in existing_data
        if is_new:
            if ip_address and check_duplicate_ip(existing_data, ip_address, code, report):
                continue
            existing_data[code] = create_new_entry(config, code=code, device_name=username if username else code, mac=mac, ipv4=ip_address, comment="PPP")

        new_values = {
            "Parent Node": parent_node,
            "MAC": mac,
            "IPv4": ip_address,
            "Download Min Mbps": rx_min,
            "Upload Min Mbps": tx_min,
            "Download Max Mbps": rx_max,
            "Upload Max Mbps": tx_max,
            "Comment": "PPP",
        }
        changes = update_entry_values(existing_data[code], new_values)
        if is_new:
            record_added(report, "PPP", code, ip_address, mac, parent_node, rx_max, tx_max)
            updated = True
        elif changes:
            record_updated(report, "PPP", code, ip_address, parent_node, changes)
            updated = True

    record_router_stat(report, router_name, "pppoe_active", len(active_codes))

    if mode == "grouped_router_root" and children is not None:
        if per_plan_node:
            generic_ppp_node = f"PPP-{router_name}"
            if generic_ppp_node in children:
                del children[generic_ppp_node]
                updated = True

            desired_plan_nodes = set(plan_totals.keys())
            stale_plan_nodes = [
                n for n in list(children.keys())
                if n.endswith(f"-{router_name}")
                and n not in desired_plan_nodes
                and not n.startswith(("DHCP-", "HS-", "PLAN-"))
            ]
            for n in stale_plan_nodes:
                del children[n]
                updated = True

            for plan_node, totals in plan_totals.items():
                down_factor, up_factor = get_plan_factors(str(totals["profile"]))
                agg_rx = safe_int_mbps(config, float(totals["rx"]) * down_factor)
                agg_tx = safe_int_mbps(config, float(totals["tx"]) * up_factor)
                new_node = {"downloadBandwidthMbps": agg_rx, "uploadBandwidthMbps": agg_tx, "type": "plan", "children": {}}
                if set_child_node(report, children, plan_node, new_node):
                    updated = True
                record_computation(
                    report,
                    mode="pppoe_per_plan",
                    node=plan_node,
                    users=totals["count"],
                    sum_down=round(float(totals["rx"]), 3),
                    sum_up=round(float(totals["tx"]), 3),
                    factor_down=down_factor,
                    factor_up=up_factor,
                    final_down=agg_rx,
                    final_up=agg_tx,
                )
        else:
            stale_plan_nodes = [
                n for n in list(children.keys())
                if n.endswith(f"-{router_name}")
                and not n.startswith(("DHCP-", "PPP-", "HS-", "PLAN-"))
            ]
            for n in stale_plan_nodes:
                del children[n]
                updated = True

            factor = float(bwd.get("pppoe_aggregate_factor", 0.30))
            cap_down = float(config.get("network", {}).get("root_download_mbps", 440))
            cap_up = float(config.get("network", {}).get("root_upload_mbps", 440))
            agg_rx = safe_int_mbps(config, min(flat_ppp_total_rx * factor, cap_down))
            agg_tx = safe_int_mbps(config, min(flat_ppp_total_tx * factor, cap_up))
            generic_ppp_node = f"PPP-{router_name}"
            new_node = {"downloadBandwidthMbps": agg_rx, "uploadBandwidthMbps": agg_tx, "type": "plan", "children": {}}
            if set_child_node(report, children, generic_ppp_node, new_node):
                updated = True
            record_computation(
                report,
                mode="flat_ppp",
                node=generic_ppp_node,
                users=len(active_codes),
                sum_down=round(flat_ppp_total_rx, 3),
                sum_up=round(flat_ppp_total_tx, 3),
                factor=factor,
                cap_down=cap_down,
                cap_up=cap_up,
                final_down=agg_rx,
                final_up=agg_tx,
            )
    else:
        record_computation(report, mode="flat_clients" if mode != "logical_core_root" else "logical_core_clients", node=root_node_name(config, router_name), users=len(active_codes))

    logger.info("[PPPoE:%s] Done. %d active users. [total: %s]", router_name, len(active_codes), elapsed(t0))
    return active_codes, updated

# =========================
# HOTSPOT
# =========================
def process_hotspot_users(config: Dict[str, Any], api, router: dict, existing_data: dict, network_config: dict, report: Dict[str, Any]) -> Tuple[set, bool]:
    if not router.get("hotspot", {}).get("enabled", False):
        return set(), False

    t0 = time.time()
    router_name = router["name"]
    hotspot_cfg = router.get("hotspot", {})
    bwd = config.get("bandwidth_defaults", {})
    mode = network_mode(config)

    include_mac = bool(hotspot_cfg.get("include_mac", True))
    per_client_down = float(hotspot_cfg.get("download_limit_mbps", bwd.get("hotspot_default_per_client_mbps", 10)))
    per_client_up = float(hotspot_cfg.get("upload_limit_mbps", bwd.get("hotspot_default_per_client_mbps", 10)))
    dl_factor = float(hotspot_cfg.get("download_factor", bwd.get("hotspot_download_factor", 1.0)))
    ul_factor = float(hotspot_cfg.get("upload_factor", bwd.get("hotspot_upload_factor", 1.0)))

    ensure_root_node(config, network_config, router_name)
    children = network_children_for_grouped(config, network_config, router_name)
    grouped_node = f"HS-{router_name}"

    updated = False
    active_codes = set()

    t_fetch = time.time()
    hotspot_users = get_resource_data(api, "/ip/hotspot/active")
    logger.info("[Hotspot:%s] Fetched %d active users. [fetch: %s]", router_name, len(hotspot_users), elapsed(t_fetch))

    for user in hotspot_users:
        mac = str(user.get("mac-address", "")).strip()
        ip = str(user.get("address", "")).strip()
        username = str(user.get("user", "")).strip()
        mac_clean = mac.replace(":", "").upper() if mac else ""

        if include_mac and mac_clean:
            code = f"HS-{mac_clean}"
            device_name = username if username else mac_clean
        else:
            if not username:
                logger.warning("Skipping Hotspot active row with no username/mac: ip=%s", ip)
                continue
            code = f"HS-{username}"
            device_name = username

        active_codes.add(code)
        parent_node = logical_parent_for_client(config, code, ip, mac) if mode == "logical_core_root" else parent_for_client(config, router_name, grouped_node)

        is_new = code not in existing_data
        if is_new:
            if ip and check_duplicate_ip(existing_data, ip, code, report):
                continue
            existing_data[code] = create_new_entry(config, code=code, device_name=device_name, mac=mac, ipv4=ip, comment="HS")

        rx_min, tx_min, rx_max, tx_max = calculate_user_min_max(config, per_client_down, per_client_up)
        new_values = {
            "Parent Node": parent_node,
            "MAC": mac,
            "IPv4": ip,
            "Download Min Mbps": rx_min,
            "Upload Min Mbps": tx_min,
            "Download Max Mbps": rx_max,
            "Upload Max Mbps": tx_max,
            "Comment": "HS",
        }
        changes = update_entry_values(existing_data[code], new_values)
        if is_new:
            record_added(report, "HS", code, ip, mac, parent_node, rx_max, tx_max)
            updated = True
        elif changes:
            record_updated(report, "HS", code, ip, parent_node, changes)
            updated = True

    record_router_stat(report, router_name, "hotspot_active", len(active_codes))

    if mode == "grouped_router_root" and children is not None:
        if active_codes:
            agg_rx = safe_int_mbps(config, len(active_codes) * per_client_down * dl_factor)
            agg_tx = safe_int_mbps(config, len(active_codes) * per_client_up * ul_factor)
            new_node = {"downloadBandwidthMbps": agg_rx, "uploadBandwidthMbps": agg_tx, "type": "site", "children": {}}
            if set_child_node(report, children, grouped_node, new_node):
                updated = True
            record_computation(report, mode="hotspot", node=grouped_node, users=len(active_codes), sum_down=len(active_codes) * per_client_down, sum_up=len(active_codes) * per_client_up, factor_down=dl_factor, factor_up=ul_factor, final_down=agg_rx, final_up=agg_tx)
        elif grouped_node in children:
            del children[grouped_node]
            updated = True

    logger.info("[Hotspot:%s] Done. %d active users. [total: %s]", router_name, len(active_codes), elapsed(t0))
    return active_codes, updated

# =========================
# DHCP
# =========================
def process_dhcp_leases(config: Dict[str, Any], api, router: dict, existing_data: dict, network_config: dict, report: Dict[str, Any]) -> Tuple[set, bool]:
    if not router.get("dhcp", {}).get("enabled", False):
        return set(), False

    t0 = time.time()
    router_name = router["name"]
    dhcp_cfg = router.get("dhcp", {})
    bwd = config.get("bandwidth_defaults", {})
    server_configs = dhcp_cfg.get("servers", [])
    net_mode = network_mode(config)

    # Backward compatibility.
    if not server_configs:
        for old_name in dhcp_cfg.get("dhcp_server", []):
            server_configs.append({
                "name": old_name,
                "mode": "per_site",
                "download_limit_mbps": dhcp_cfg.get("download_limit_mbps", bwd.get("dhcp_default_per_client_mbps", 15)),
                "upload_limit_mbps": dhcp_cfg.get("upload_limit_mbps", bwd.get("dhcp_default_per_client_mbps", 15)),
                "download_factor": bwd.get("dhcp_download_factor", 0.5),
                "upload_factor": bwd.get("dhcp_upload_factor", 0.5),
            })

    ensure_root_node(config, network_config, router_name)
    children = network_children_for_grouped(config, network_config, router_name)

    updated = False
    active_codes = set()

    t_fetch = time.time()
    all_leases = get_resource_data(api, "/ip/dhcp-server/lease")
    logger.info("[DHCP:%s] Fetched %d leases. [fetch: %s]", router_name, len(all_leases), elapsed(t_fetch))
    desired_nodes = set()

    for server_cfg in server_configs:
        server_name = server_cfg.get("name")
        if not server_name:
            continue

        mode = str(server_cfg.get("mode", "per_site")).strip().lower()
        dl_factor = float(server_cfg.get("download_factor", bwd.get("dhcp_download_factor", 0.5)))
        ul_factor = float(server_cfg.get("upload_factor", bwd.get("dhcp_upload_factor", 0.5)))

        server_leases = [
            lease for lease in all_leases
            if lease.get("server") == server_name
            and lease.get("mac-address")
            and (lease.get("active-address") or lease.get("address"))
        ]

        if mode == "per_site":
            per_client_down = float(server_cfg.get("download_limit_mbps", server_cfg.get("default_plan_down_mbps", bwd.get("dhcp_default_per_client_mbps", 15))))
            per_client_up = float(server_cfg.get("upload_limit_mbps", server_cfg.get("default_plan_up_mbps", bwd.get("dhcp_default_per_client_mbps", 15))))
            grouped_node = f"DHCP-{server_name}-{router_name}"
            desired_nodes.add(grouped_node)

            for lease in server_leases:
                mac = str(lease.get("mac-address", "")).strip()
                ip = str(lease.get("active-address") or lease.get("address") or "").strip()
                hostname = str(lease.get("host-name", "")).strip()
                mac_clean = mac.replace(":", "").upper()
                code = f"DHCP-{hostname}" if hostname else f"DHCP-{mac_clean}"
                device_name = hostname if hostname else mac_clean
                active_codes.add(code)

                parent_node = logical_parent_for_client(config, code, ip, mac) if net_mode == "logical_core_root" else parent_for_client(config, router_name, grouped_node)

                is_new = code not in existing_data
                if is_new:
                    if ip and check_duplicate_ip(existing_data, ip, code, report):
                        continue
                    existing_data[code] = create_new_entry(config, code=code, device_name=device_name, mac=mac, ipv4=ip, comment="DHCP")

                rx_min, tx_min, rx_max, tx_max = calculate_user_min_max(config, per_client_down, per_client_up)
                new_values = {
                    "Parent Node": parent_node,
                    "MAC": mac,
                    "IPv4": ip,
                    "Download Min Mbps": rx_min,
                    "Upload Min Mbps": tx_min,
                    "Download Max Mbps": rx_max,
                    "Upload Max Mbps": tx_max,
                    "Comment": hostname if hostname else "DHCP",
                }
                changes = update_entry_values(existing_data[code], new_values)
                if is_new:
                    record_added(report, "DHCP", code, ip, mac, parent_node, rx_max, tx_max)
                    updated = True
                elif changes:
                    record_updated(report, "DHCP", code, ip, parent_node, changes)
                    updated = True

            if net_mode == "grouped_router_root" and children is not None:
                if server_leases:
                    agg_rx = safe_int_mbps(config, len(server_leases) * per_client_down * dl_factor)
                    agg_tx = safe_int_mbps(config, len(server_leases) * per_client_up * ul_factor)
                    new_node = {"downloadBandwidthMbps": agg_rx, "uploadBandwidthMbps": agg_tx, "type": "site", "children": {}}
                    if set_child_node(report, children, grouped_node, new_node):
                        updated = True
                    record_computation(report, mode="dhcp_per_site", node=grouped_node, users=len(server_leases), sum_down=len(server_leases) * per_client_down, sum_up=len(server_leases) * per_client_up, factor_down=dl_factor, factor_up=ul_factor, final_down=agg_rx, final_up=agg_tx)
                elif grouped_node in children:
                    del children[grouped_node]
                    updated = True

        elif mode == "per_plan":
            plan_totals: Dict[str, Dict[str, Any]] = {}

            for lease in server_leases:
                mac = str(lease.get("mac-address", "")).strip()
                ip = str(lease.get("active-address") or lease.get("address") or "").strip()
                hostname = str(lease.get("host-name", "")).strip()
                mac_clean = mac.replace(":", "").upper()
                lease_comment = str(lease.get("comment", "")).strip()
                parsed = parse_comment_bandwidth(lease_comment)
                if not parsed:
                    logger.warning("Skipping DHCP lease on %s with no plan: ip=%s comment='%s'", server_name, ip, lease_comment)
                    continue

                base_down, base_up = parsed
                name_match = RE_PLAN_NAME.match(lease_comment)
                client_name = name_match.group(1).strip() if name_match else hostname
                code = f"DHCP-{client_name}" if client_name else f"DHCP-{mac_clean}"
                device_name = client_name if client_name else mac_clean
                active_codes.add(code)

                plan_label = f"{int(base_down)}M" if base_down == base_up else f"{int(base_down)}M-{int(base_up)}M"
                grouped_node = f"PLAN-DHCP-{plan_label}-{router_name}"
                desired_nodes.add(grouped_node)
                parent_node = logical_parent_for_client(config, code, ip, mac) if net_mode == "logical_core_root" else parent_for_client(config, router_name, grouped_node)

                if net_mode != "logical_core_root":
                    plan_totals.setdefault(grouped_node, {"rx": 0.0, "tx": 0.0, "count": 0})
                    plan_totals[grouped_node]["rx"] += base_down
                    plan_totals[grouped_node]["tx"] += base_up
                    plan_totals[grouped_node]["count"] += 1

                is_new = code not in existing_data
                if is_new:
                    if ip and check_duplicate_ip(existing_data, ip, code, report):
                        continue
                    existing_data[code] = create_new_entry(config, code=code, device_name=device_name, mac=mac, ipv4=ip, comment="DHCP")

                rx_min, tx_min, rx_max, tx_max = calculate_user_min_max(config, base_down, base_up)
                new_values = {
                    "Parent Node": parent_node,
                    "MAC": mac,
                    "IPv4": ip,
                    "Download Min Mbps": rx_min,
                    "Upload Min Mbps": tx_min,
                    "Download Max Mbps": rx_max,
                    "Upload Max Mbps": tx_max,
                    "Comment": client_name if client_name else "DHCP",
                }
                changes = update_entry_values(existing_data[code], new_values)
                if is_new:
                    record_added(report, "DHCP", code, ip, mac, parent_node, rx_max, tx_max)
                    updated = True
                elif changes:
                    record_updated(report, "DHCP", code, ip, parent_node, changes)
                    updated = True

            if net_mode == "grouped_router_root" and children is not None:
                for plan_node, totals in plan_totals.items():
                    agg_rx = safe_int_mbps(config, float(totals["rx"]) * dl_factor)
                    agg_tx = safe_int_mbps(config, float(totals["tx"]) * ul_factor)
                    new_node = {"downloadBandwidthMbps": agg_rx, "uploadBandwidthMbps": agg_tx, "type": "plan", "children": {}}
                    if set_child_node(report, children, plan_node, new_node):
                        updated = True
                    record_computation(report, mode="dhcp_per_plan", node=plan_node, users=totals["count"], sum_down=round(float(totals["rx"]), 3), sum_up=round(float(totals["tx"]), 3), factor_down=dl_factor, factor_up=ul_factor, final_down=agg_rx, final_up=agg_tx)

    if net_mode == "grouped_router_root" and children is not None:
        stale_nodes = [
            n for n in list(children.keys())
            if (n.startswith("DHCP-") or n.startswith("PLAN-"))
            and n.endswith(f"-{router_name}")
            and n not in desired_nodes
        ]
        for n in stale_nodes:
            del children[n]
            logger.info("Removed stale DHCP-derived node %s", n)
            updated = True

    record_router_stat(report, router_name, "dhcp_active", len(active_codes))
    logger.info("[DHCP:%s] Done. %d active leases. [total: %s]", router_name, len(active_codes), elapsed(t0))
    return active_codes, updated

# =========================
# CLEANUP / LIBREQOS
# =========================
def remove_inactive_entries(config: Dict[str, Any], existing_data: dict, active_codes: set, report: Dict[str, Any]) -> bool:
    if not bool(config.get("runtime", {}).get("remove_inactive_entries", True)):
        return False
    updated = False
    for code in list(existing_data.keys()):
        comment = str(existing_data[code].get("Comment", "")).strip().lower()
        if code not in active_codes and comment != "static":
            row = existing_data[code]
            record_removed(report, code, row.get("IPv4", ""), row.get("Parent Node", ""), row.get("Comment", ""))
            logger.info("Removing inactive entry %s", code)
            del existing_data[code]
            updated = True
    return updated


def execute_libreqos_update(config: Dict[str, Any], report: Dict[str, Any]) -> Dict[str, Any]:
    libcfg = config.get("libreqos", {})
    if not libcfg.get("enabled", True):
        result = {"status": "disabled"}
        report["libreqos"] = result
        return result

    cmd = str(libcfg.get("command", "/opt/libreqos/src/LibreQoS.py"))
    timeout = int(libcfg.get("timeout_seconds", 300))
    run_cmd = [cmd, "--updateonly"]
    if libcfg.get("sudo", True):
        run_cmd = ["/usr/bin/sudo"] + run_cmd

    t0 = time.time()
    try:
        logger.info("Running LibreQoS update...")
        proc = subprocess.run(run_cmd, capture_output=True, text=True, timeout=timeout, check=True)
        if proc.stdout.strip():
            logger.info("LibreQoS stdout:\n%s", proc.stdout.strip())
        if proc.stderr.strip():
            logger.warning("LibreQoS stderr:\n%s", proc.stderr.strip())
        result = {"status": "success", "duration": elapsed(t0), "stdout": proc.stdout.strip(), "stderr": proc.stderr.strip()}
    except subprocess.TimeoutExpired:
        result = {"status": "timeout", "duration": elapsed(t0)}
        report_error(report, "LibreQoS", "update timed out")
    except subprocess.CalledProcessError as e:
        result = {"status": "failed", "duration": elapsed(t0), "stdout": e.stdout, "stderr": e.stderr}
        report_error(report, "LibreQoS", e.stderr or e.stdout or e)
    except Exception as e:
        result = {"status": "error", "duration": elapsed(t0), "error": str(e)}
        report_error(report, "LibreQoS", e)

    report["libreqos"] = result
    return result


# =========================
# MAIN LOOP
# =========================
def run_one_cycle(config: Dict[str, Any]) -> Tuple[Dict[str, Any], bool, bool, int]:
    cycle_start = time.time()
    report = new_report()
    run_preflight(config, report)

    routers = config.get("routers", [])

    # Load persistent LCORE assignments before reading MikroTik data.
    # This keeps client parent nodes sticky between service restarts.
    load_logical_core_state(config)

    existing_data = read_shaped_devices_csv(config)
    network_config = update_network_json_skeleton(config, routers)

    all_active_codes = set()
    any_updates = False
    router_processed_successfully = False

    for router in routers:
        router_name = router.get("name", "unknown")
        router_start = time.time()
        logger.info("Processing router %s (per_plan_node=%s)", router_name, router.get("pppoe", {}).get("per_plan_node", False))

        pool, api = connect_to_router(router, report)
        if not api:
            continue

        try:
            router_active_codes = set()
            router_updated = False

            for processor in (process_pppoe_users, process_hotspot_users, process_dhcp_leases):
                active_codes, updated = processor(config, api, router, existing_data, network_config, report)
                router_active_codes.update(active_codes)
                router_updated = router_updated or updated

            all_active_codes.update(router_active_codes)
            any_updates = any_updates or router_updated
            router_processed_successfully = True
            record_router_stat(report, router_name, "active_total", len(router_active_codes))
            record_router_stat(report, router_name, "duration", elapsed(router_start))
            logger.info("Router %s done. %d active entries. [%s]", router_name, len(router_active_codes), elapsed(router_start))

        except Exception as e:
            report_error(report, f"router:{router_name}", e)
        finally:
            try:
                pool.disconnect()
            except Exception:
                pass

    if router_processed_successfully:
        if remove_inactive_entries(config, existing_data, all_active_codes, report):
            any_updates = True

        # After inactive clients are removed, rebalance only a limited number of
        # sticky assignments if the buckets drift beyond the configured threshold.
        if rebalance_logical_assignments(config, existing_data, all_active_codes, report):
            any_updates = True
    else:
        logger.warning("Skipping inactive cleanup -- no router completed successfully.")

    # Detect pure network.json skeleton changes, e.g. CPU core count/reserve changed
    # and LCORE bucket count must be regenerated.
    try:
        if read_file_text(get_network_json(config), "") != render_network_json(network_config):
            any_updates = True
    except Exception:
        pass

    files_changed = False
    csv_changed = False
    json_changed = False
    state_changed = False

    if any_updates:
        try:
            csv_changed = write_shaped_devices_csv_if_changed(config, existing_data)
            json_changed = write_json_file_if_changed(get_network_json(config), network_config)
            state_changed = write_logical_core_state_if_changed(config)
            files_changed = csv_changed or json_changed
            if files_changed:
                logger.info("File changes detected. Running LibreQoS update.")
                execute_libreqos_update(config, report)
            else:
                logger.info("In-memory/state updates only -- LibreQoS data files unchanged. Skipping LibreQoS.")
                report["libreqos"] = {"status": "skipped_no_file_changes"}
        except Exception as e:
            report_error(report, "write_files", e)
    else:
        # Still allow first-run state-file creation if assignments were changed
        # but no LibreQoS CSV/network changes were needed.
        try:
            state_changed = write_logical_core_state_if_changed(config)
        except Exception as e:
            report_error(report, "write_logical_state", e)
        logger.info("No changes detected.")
        report["libreqos"] = {"status": "skipped_no_changes"}

    last_had_updates = any_updates
    next_sleep = int(config.get("runtime", {}).get("scan_interval_active" if last_had_updates else "scan_interval_idle", 120))

    report["telemetry"]["csv_changed"] = csv_changed
    report["telemetry"]["network_json_changed"] = json_changed
    report["telemetry"]["logical_state_changed"] = state_changed
    report["telemetry"]["cycle_duration"] = elapsed(cycle_start)

    return report, any_updates, files_changed, next_sleep

def should_send_telegram(config: Dict[str, Any], report: Dict[str, Any], any_updates: bool, files_changed: bool) -> bool:
    """
    Strict Telegram rule:
      - Send on errors if notify_on_error=true.
      - Send on real file changes if notify_on_changes=true.
      - Do NOT send on normal no-change scans.
      - Do NOT send just because preflight passed/ran.
      - Do NOT send just because service started.
    """
    if not telegram_enabled(config):
        return False

    tg = config.get("telegram", {})

    if report.get("errors") and tg.get("notify_on_error", True):
        return True

    if files_changed and tg.get("notify_on_changes", True):
        return True

    return False


def main() -> None:
    # First load config and setup logging.
    config = load_config()
    setup_logging(config)
    logger.info("MikroTik to LibreQoS sync service started")

    started_notice_sent = False

    while True:
        try:
            # Reload config every cycle so tuning config.json does not need service restart.
            config = load_config()
            setup_logging(config)

            # No start notification. Telegram should only notify on real changes or errors.
            started_notice_sent = True

            report, any_updates, files_changed, next_sleep = run_one_cycle(config)

            if should_send_telegram(config, report, any_updates, files_changed):
                msg = build_telegram_message(config, report, files_changed, any_updates, next_sleep)
                telegram_send(config, msg)

            logger.info("Scan complete. [%s] Next scan in %ds.", report.get("telemetry", {}).get("cycle_duration", "n/a"), next_sleep)
            time.sleep(next_sleep)

        except KeyboardInterrupt:
            logger.info("Stopping service.")
            break
        except Exception as e:
            logger.error("Main loop error: %s", e)
            try:
                config = load_config()
                if telegram_enabled(config) and config.get("telegram", {}).get("notify_on_error", True):
                    telegram_send(config, f"ALERT: <b>LQoSync main loop error</b>\n<code>{esc(e)}</code>")
                retry = int(config.get("runtime", {}).get("error_retry_interval", 30))
            except Exception:
                retry = 30
            time.sleep(retry)


if __name__ == "__main__":
    main()
