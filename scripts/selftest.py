#!/usr/bin/env python3
"""Offline self-test for the Rust run-cycle authority."""
from __future__ import annotations

import csv
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

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

try:
    from auth.users import add_user, update_user, set_user_password, delete_user, list_users, authenticate
except Exception:
    add_user = update_user = set_user_password = delete_user = list_users = authenticate = None


def core_bin() -> str:
    candidates = [
        os.environ.get("LQOSYNC_CORE_BIN"),
        str(ROOT / "rust/lqosync-core/target/release/lqosync-core"),
        shutil.which("lqosync-core"),
    ]
    for candidate in candidates:
        if candidate and Path(candidate).is_file() and os.access(candidate, os.X_OK):
            return candidate
    raise RuntimeError(
        "lqosync-core binary not found. Set LQOSYNC_CORE_BIN or build rust/lqosync-core first."
    )


def fixture_results(router_name: str) -> list[dict]:
    return [
        {
            "router": router_name,
            "source": "pppoe",
            "path": "/ppp/active",
            "status": "ok",
            "rows": [
                {
                    "name": "juan",
                    "address": "10.0.100.10",
                    "caller-id": "AA:BB:CC:DD:EE:01",
                }
            ],
        },
        {
            "router": router_name,
            "source": "pppoe",
            "path": "/ppp/secret",
            "status": "ok",
            "rows": [
                {
                    "name": "juan",
                    "disabled": "false",
                    "inactive": "false",
                    "profile": "Tier-15M",
                    "comment": "",
                }
            ],
        },
        {
            "router": router_name,
            "source": "pppoe",
            "path": "/ppp/profile",
            "status": "ok",
            "rows": [{"name": "Tier-15M", "rate-limit": "15M/15M"}],
        },
        {
            "router": router_name,
            "source": "dhcp",
            "path": "/ip/dhcp-server/lease",
            "status": "ok",
            "rows": [
                {
                    "server": "LAN",
                    "mac-address": "AA:BB:CC:DD:EE:02",
                    "active-address": "10.17.0.20",
                    "host-name": "mesh",
                    "status": "bound",
                }
            ],
        },
        {
            "router": router_name,
            "source": "dhcp",
            "path": "/ip/dhcp-server",
            "status": "ok",
            "rows": [{"name": "LAN", "interface": "bridge"}],
        },
    ]


def write_config(path: Path, config: dict) -> None:
    path.write_text(json.dumps(config, indent=2) + "\n", encoding="utf-8")


def run_authority(binary: str, config_path: Path, mode: str, results: list[dict]) -> dict:
    request = {
        "version": "1",
        "op": "run-rust-cycle-authority",
        "payload": {
            "config_path": str(config_path),
            "mode": mode,
            "execute": True,
            "results": results,
        },
    }
    completed = subprocess.run(
        [binary],
        input=json.dumps(request),
        text=True,
        capture_output=True,
        check=False,
    )
    if not completed.stdout.strip():
        raise AssertionError(
            f"lqosync-core returned no JSON for mode={mode}: stderr={completed.stderr.strip()}"
        )
    data = json.loads(completed.stdout)
    if completed.returncode not in (0, 2):
        raise AssertionError(
            f"unexpected lqosync-core exit code {completed.returncode}: {completed.stderr.strip()}"
        )
    return data


def base_config(csv_path: Path, net_path: Path, state_path: Path, log_path: Path, journal_path: Path) -> dict:
    return {
        "app": {"auto_apply": False, "backup_before_apply": False},
        "defaults": {
            "default_pppoe_rate": "10M/10M",
            "default_dhcp_per_client_mbps": 15,
            "min_rate_percentage": 0.5,
        },
        "paths": {
            "shaped_devices_csv": str(csv_path),
            "network_json": str(net_path),
            "backup_dir": str(csv_path.parent / "backups"),
            "log_file": str(log_path),
            "runtime_state": str(state_path),
            "transaction_journal": str(journal_path),
        },
        "scheduler": {"enabled": False, "engine": "rust", "allow_python_scheduler": False},
        "libreqos": {
            "cmd": "/opt/libreqos/src/LibreQoS.py",
            "args": ["--updateonly"],
            "timeout_seconds": 1,
            "run_only_when_files_changed": True,
            "sudo": False,
            "retry_if_last_apply_failed": True,
        },
        "rust_core": {
            "enabled": True,
            "prefer_daemon": False,
            "native_dry_run_preview_enabled": True,
            "native_run_cycle_authority_enabled": True,
            "native_run_cycle_authority_python_fallback": False,
            "execute_apply_manifest": True,
            "allow_rust_file_writes": True,
            "allow_rust_libreqos_apply": False,
            "append_transaction_journal": False,
            "allow_transaction_journal_writes": False,
            "allow_dry_run_journal_entries": False,
            "include_rehearsal_journal_entries": False,
            "full_rust_backend_authority": False,
        },
        "routers": [
            {
                "name": "RB5k9-Distro",
                "enabled": True,
                "address": "127.0.0.1",
                "port": 8728,
                "username": "x",
                "password": "x",
                "root_download_mbps": 115,
                "root_upload_mbps": 115,
                "pppoe": {
                    "enabled": True,
                    "per_plan_node": True,
                    "factor_rules": [
                        {"max_plan_mbps": 15, "download_factor": 0.31, "upload_factor": 0.31},
                        {"max_plan_mbps": 9999, "download_factor": 1.0, "upload_factor": 1.0},
                    ],
                },
                "dhcp": {
                    "enabled": True,
                    "servers": [
                        {
                            "name": "LAN",
                            "enabled": True,
                            "mode": "per_site",
                            "default_plan_down_mbps": 15,
                            "default_plan_up_mbps": 15,
                            "download_factor": 0.3,
                            "upload_factor": 0.3,
                        }
                    ],
                },
                "hotspot": {"enabled": False},
            }
        ],
    }


def result_dict(response: dict) -> dict:
    return response.get("result") if isinstance(response.get("result"), dict) else {}


def main():
    binary = core_bin()
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        csv_path = td / "ShapedDevices.csv"
        net_path = td / "network.json"
        cfg_path = td / "config.json"
        state_path = td / "state.json"
        log_path = td / "test.log"
        journal_path = td / "transaction_journal.jsonl"
        csv_path.write_text(",".join(FIELDNAMES) + "\n", encoding="utf-8")
        net_path.write_text("{}\n", encoding="utf-8")

        cfg = base_config(csv_path, net_path, state_path, log_path, journal_path)
        write_config(cfg_path, cfg)
        results = fixture_results("RB5k9-Distro")

        dry = result_dict(run_authority(binary, cfg_path, "dry_run", results))
        assert dry.get("status") == "dry_run_complete", dry
        assert dry.get("files_changed") is True, dry
        assert "juan" not in csv_path.read_text(encoding="utf-8"), "dry-run must not write"

        applied = result_dict(run_authority(binary, cfg_path, "manual", results))
        assert applied.get("status") == "success", applied
        text = csv_path.read_text(encoding="utf-8")
        assert "juan" in text and "DHCP-mesh" in text, text
        net = json.loads(net_path.read_text(encoding="utf-8"))
        assert "RB5k9-Distro" in net
        assert "Tier-15M-RB5k9-Distro" in net["RB5k9-Distro"]["children"]
        assert "DHCP-LAN-RB5k9-Distro" in net["RB5k9-Distro"]["children"]

        if add_user:
            users_path = td / "users.json"
            add_user("viewer1", "viewpass", "viewer", users_path)
            assert authenticate("viewer1", "viewpass") is None
            assert any(
                u["username"] == "viewer1" and u["role"] == "viewer"
                for u in list_users(users_path)
            )
            update_user("viewer1", "viewer2", "admin", users_path)
            assert any(
                u["username"] == "viewer2" and u["role"] == "admin"
                for u in list_users(users_path)
            )
            set_user_password("viewer2", "newpass", users_path)
            delete_user("viewer2", current_username="admin", path=users_path)
            assert not any(u["username"] == "viewer2" for u in list_users(users_path))

        for mode, expected_parent, expect_empty_network in (
            ("flat_router_root", "RB5k9-Distro", False),
            ("flat_no_parent", "", True),
        ):
            csv_path.write_text(",".join(FIELDNAMES) + "\n", encoding="utf-8")
            net_path.write_text("{}\n", encoding="utf-8")
            cfg["network_mode"] = mode
            write_config(cfg_path, cfg)
            flat_res = result_dict(run_authority(binary, cfg_path, "manual", results))
            assert flat_res.get("status") == "success", flat_res
            rows = list(csv.DictReader(csv_path.open(newline="", encoding="utf-8")))
            assert rows, mode
            assert {row.get("Parent Node", "") for row in rows} == {expected_parent}, rows
            net2 = json.loads(net_path.read_text(encoding="utf-8"))
            if expect_empty_network:
                assert net2 == {}, net2
            else:
                assert net2.get("RB5k9-Distro", {}).get("children") == {}, net2
    print("LQoSync self-test passed")


if __name__ == "__main__":
    main()
