#!/usr/bin/env python3
"""Capture the real TUI in Zutty/Xvfb with isolated synthetic input files.
Requires Xvfb, Zutty, libX11, libXtst, and Pillow. Never reads production config.
"""
import argparse
import ctypes as C
import csv
import json
import math
import os
from pathlib import Path
import subprocess
import tempfile
import time
from PIL import ImageGrab


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--screen', type=int, choices=range(1, 8), default=1)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='lqosync3-capture-') as folder:
        path = Path(folder)
        config = path / 'config.toml'
        config.write_text(f'''[runtime]
mode = "apply"
state_dir = "{path}"
[libreqos]
target = "v3_pair"
directory = "{path}"
state_directory = "{path}"
v3_config = "{path}/lqos.conf"
v3_analytics_socket = "{path}/analytics.sock"
dpkg_status = "{path}/dpkg"
[tui]
service_unit = "lqosync3-capture-fixture.service"
lqosd_units = []
refresh_ms = 1000
[[routers]]
name = "EDGE-01"
address = "192.0.2.101"
username = "fixture"
password_file = "{path}/fixture.password"
[routers.pppoe]
enabled = true
[routers.hotspot]
enabled = true
[routers.dhcp]
enabled = true
''')
        (path / 'fixture.password').write_text('synthetic-not-a-real-password')
        (path / 'fixture.password').chmod(0o600)
        (path / 'network.json').write_text(json.dumps({'AP-NORTH': {'downloadBandwidthMbps': 1000, 'uploadBandwidthMbps': 1000}}))
        (path / 'state.json').write_text(json.dumps({'version': 1, 'circuits': {}, 'nodes': {}, 'routers': {}}))
        with (path / 'ShapedDevices.csv').open('w') as f:
            writer = csv.writer(f)
            writer.writerow(['Circuit ID', 'Circuit Name', 'Device ID', 'Device Name', 'Parent Node', 'MAC', 'IPv4', 'IPv6', 'Download Min Mbps', 'Upload Min Mbps', 'Download Max Mbps', 'Upload Max Mbps', 'Comment'])
            for i in range(32):
                writer.writerow([f'SUB-{i:04}', f'Subscriber {i:02}', f'd{i}', f'Client {i}', 'AP-NORTH', f'02:00:00:00:00:{i:02x}', f'192.0.2.{i+1}', '', 5, 2, 100, 50, 'capture fixture'])
        (path / 'state.json').write_text(json.dumps({'version':1,'circuits':{f'Subscriber {i:02}':{'router':'EDGE-01','source':'ppp' if i%2 else 'dhcp','last_seen':int(time.time()),'row':{}} for i in range(32)},'nodes':{},'routers':{}}))
        display = next(f':{i}' for i in range(190, 220) if not Path(f'/tmp/.X11-unix/X{i}').exists())
        processes = []
        try:
            processes.append(subprocess.Popen(['Xvfb', display, '-screen', '0', '2164x724x24', '-nolisten', 'tcp'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
            time.sleep(.5)
            env = {**os.environ, 'DISPLAY': display, 'LIBGL_ALWAYS_SOFTWARE': '1', 'TERM': 'xterm-256color', 'COLORTERM': 'truecolor'}
            env.pop('NO_COLOR', None)
            if args.screen == 7:
                # Isolated journalctl provider. Do not read or write the host journal.
                tools_dir = path / 'bin'; tools_dir.mkdir()
                messages = [
                    'INFO reload request received', 'INFO reload validating input',
                    'INFO reload source files read', 'WARN reload delayed; retry scheduled',
                    'ERROR reload failed: source read timeout on EDGE-01',
                    'INFO reload retry started', 'INFO reload validation passed',
                    'INFO reload complete; configuration accepted',
                ]
                logs = [f'2026-11-09T10:14:{38+i:02}+0800 fixture lqosync3[321]: {m}' for i,m in enumerate(messages)]
                provider = tools_dir / 'journalctl'
                provider.write_text('#!/usr/bin/python3\nimport time\nfor line in '+repr(logs)+':\n print(line, flush=True)\ntime.sleep(120)\n')
                provider.chmod(0o700)
                env['PATH'] = str(tools_dir) + os.pathsep + env['PATH']

            # Keep snapshots fresh while the terminal starts; all data stays in this directory.
            def snapshots():
                now = int(time.time())
                status = {'updated': now, 'target': 'v3_pair', 'mode': 'apply', 'reload': 'imported by LibreQoS', 'files_changed': False, 'telegram': 'disabled', 'routers': [{'name': 'EDGE-01', 'state': 'online', 'rows': {'/ppp/active': 12, '/ppp/secret': 18, '/ppp/profile': 1, '/ip/hotspot/active': 2, '/ip/dhcp-server': 1, '/ip/dhcp-server/lease': 16}, 'read_ms': 184}], 'report': {'alerts': [], 'counts': {}}}
                (path / 'status.json').write_text(json.dumps(status))
                subscribers = []
                for i in range(32):
                    source = 'ppp' if i % 2 else 'dhcp'
                    state = ('online' if i < 24 else 'offline') if source == 'ppp' else ('bound' if i < 24 else 'unbound')
                    subscribers.append({'key': f'{source}-{i}', 'source': source, 'name': f'Client {i}', 'circuit': f'Subscriber {i:02}', 'ipv4': f'192.0.2.{i+1}', 'mac': f'02:00:00:00:00:{i:02x}', 'state': state})
                for i in range(2):
                    subscribers.append({'key': f'ppp-offline-{i}', 'source':'ppp', 'name':f'Offline account {i}', 'circuit':f'Offline account {i}', 'ipv4':'', 'mac':'', 'state':'offline'})
                    subscribers.append({'key': f'hotspot-{i}', 'source':'hotspot', 'name':f'Guest {i}', 'circuit':f'Hotspot guest {i}', 'ipv4':f'198.51.100.{i+1}', 'mac':'', 'state':'online'})
                collection = {'version':1,'pid':1,'updated':now,'fresh_for':150,'routers':[{'name':'EDGE-01','state':'online','observed_at':now,'sources':{'ppp':{'enabled':True,'records':12,'online':12,'offline':6},'hotspot':{'enabled':True,'records':2,'online':2,'offline':None},'dhcp':{'enabled':True,'records':16,'online':12,'offline':4}},'subscribers':subscribers}]}
                (path / 'collection.json').write_text(json.dumps(collection))

                points = [{'at': now-3600+i*2, 'down_mbps': 800+math.sin(i/10)*80, 'up_mbps': 95+math.sin(i/15)*12} for i in range(1801)]
                report = {'version': 1, 'updated': now, 'stale': False, 'reason': None, 'workers': [{'queue_id': i, 'busy_ppm': 431000-i*70000, 'backlog_mean': 8-i} for i in range(4)], 'pressure': {'diagnostics': {'pool_total_mbufs': 1000, 'pool_free_mbufs': 840, 'ring_count': 2, 'ring_capacity': 200, 'ring_used': 25, 'rings_over_75': 0}, 'rx_nombuf_delta': 0, 'pool_drop_delta': 0}, 'traffic': points, 'queue': {'totals': {'drop_pkts': 103, 'aqm_marks': 205, 'backlog_bytes': 4096}, 'drop_delta': 3, 'aqm_delta': 5}}
                (path / 'worker-utilization.json').write_text(json.dumps(report))
            snapshots()
            log = (path / 'zutty.log').open('w')
            processes.append(subprocess.Popen(['zutty', '-geometry', '240x40', '-font', '9x18', '-title', 'LQoSync3 - SYNTHETIC CAPTURE', '-e', str(root / 'target/debug/lqosync3'), 'tui', '--config', str(config)], env=env, stdout=log, stderr=log))
            time.sleep(1.5)
            x = C.CDLL('libX11.so.6'); xt = C.CDLL('libXtst.so.6')
            x.XOpenDisplay.restype = C.c_void_p; x.XOpenDisplay.argtypes = [C.c_char_p]
            d = x.XOpenDisplay(display.encode())
            if not d:
                raise RuntimeError('X display unavailable')
            x.XDefaultRootWindow.argtypes = [C.c_void_p]; x.XDefaultRootWindow.restype = C.c_ulong
            x.XQueryTree.argtypes = [C.c_void_p,C.c_ulong,C.POINTER(C.c_ulong),C.POINTER(C.c_ulong),C.POINTER(C.POINTER(C.c_ulong)),C.POINTER(C.c_uint)]
            root_window=x.XDefaultRootWindow(d); returned=C.c_ulong(); parent=C.c_ulong(); children=C.POINTER(C.c_ulong)(); count=C.c_uint()
            x.XQueryTree(d,root_window,C.byref(returned),C.byref(parent),C.byref(children),C.byref(count))
            if count.value == 0:
                raise RuntimeError('Terminal did not open: '+(path/'zutty.log').read_text())
            x.XSetInputFocus.argtypes=[C.c_void_p,C.c_ulong,C.c_int,C.c_ulong]; x.XSetInputFocus(d,children[0],1,0)
            x.XStringToKeysym.argtypes=[C.c_char_p];x.XStringToKeysym.restype=C.c_ulong
            x.XKeysymToKeycode.argtypes=[C.c_void_p,C.c_ulong];x.XKeysymToKeycode.restype=C.c_uint
            xt.XTestFakeKeyEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
            x.XFlush.argtypes=[C.c_void_p]
            def key(text):
                code=x.XKeysymToKeycode(d,x.XStringToKeysym(text.encode()))
                xt.XTestFakeKeyEvent(d,code,1,0);xt.XTestFakeKeyEvent(d,code,0,0);x.XFlush(d)
            snapshots();key(str(args.screen));time.sleep(1)
            if args.screen == 7:
                key('f'); key('slash')
                for char in 'reload': key(char)
                key('Return'); key('Home')
                for _ in range(4): key('Down')
                time.sleep(.4)

            if processes[-1].poll() is not None:
                raise RuntimeError((path/'zutty.log').read_text())
            ImageGrab.grab(xdisplay=display).save(args.output)
            key('q');time.sleep(.2)
            x.XFree.argtypes=[C.c_void_p];x.XFree(children)
            x.XCloseDisplay.argtypes=[C.c_void_p];x.XCloseDisplay(d)
            print(f'Saved actual terminal capture with synthetic data: {args.output}')
        finally:
            for proc in reversed(processes):
                if proc.poll() is None:
                    proc.terminate()
                    try: proc.wait(timeout=3)
                    except subprocess.TimeoutExpired: proc.kill();proc.wait()

if __name__ == '__main__':
    main()
