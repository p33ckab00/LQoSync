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
    parser.add_argument('--traffic-source', choices=['combined','actual','mapped','unmapped'], default='combined')
    parser.add_argument('--worker-phases', action='store_true')
    parser.add_argument('--worker-health', action='store_true')
    parser.add_argument('--subscriber-stats', action='store_true')
    parser.add_argument('--queue-diagnostics', action='store_true')
    args = parser.parse_args()
    if args.screen != 1 and args.traffic_source != 'combined':
        parser.error('--traffic-source applies to Dashboard only')
    if args.worker_phases and args.screen != 5:
        parser.error('--worker-phases applies to Workers only')
    if args.worker_health and (args.screen != 5 or args.worker_phases):
        parser.error('--worker-health requires Workers and cannot combine with --worker-phases')
    if (args.subscriber_stats or args.queue_diagnostics) and args.screen != 2:
        parser.error('Subscriber detail options apply to Subscribers only')
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
                for worker in report['workers']:
                    worker.update(idle_ppm=1000000-worker['busy_ppm'], laps_per_s=120000,
                                  busy_laps=218000000, cycles_per_busy_lap=285000,
                                  phase_cycles=[772000000000,146000000000,231000000000,0,362000000000,
                                                53000000000,1216000000000,6637000000000,0,56105000000000])
                subscriber_stats={'version':1,'updated':now,'stale':False,'reason':None,'hosts':{}}
                for i in range(32):
                    subscriber_stats['hosts'][f'SUB-{i:04}']={f'192.0.2.{i+1}':{
                        'actual':{'download_bytes':1500000000,'upload_bytes':230000000},
                        'down_mbps':12.375,'up_mbps':1.25,'rtt_ms':10.5,'rtt_down_ms':12.0,'rtt_up_ms':3.0,
                        'retransmits':3,'retransmits_down':2,'retransmits_up':1,'seen_age_ms':80}}
                (path/'subscriber-telemetry.json').write_text(json.dumps(subscriber_stats))
                report['health'] = {'updated':now, 'stale':False, 'reason':None,
                                    'summary':{'status':'degraded','version':'0.1.0','uptime_ms':44966000,
                                               'topology_tier':None,'classification_empty':False,
                                               'degraded_subsystems':['stats_summary'],'worker_count':4,
                                               'missing_rollups':0,'max_rollup_age_ms':93,'aqm_degraded_workers':0}}
                report['classification'] = {
                    'mapped': {'bytes': {'download_bytes': 3000000000, 'upload_bytes': 400000000},
                               'history': [dict(v, down_mbps=v['down_mbps']*1.02, up_mbps=v['up_mbps']*1.02) for v in points]},
                    'unmapped': {'bytes': {'download_bytes': 2000000, 'upload_bytes': 3000000},
                                 'history': [dict(v, down_mbps=.04+math.sin(i/12)*.01, up_mbps=.004+math.sin(i/15)*.001) for i,v in enumerate(points)]},
                }
                (path / 'worker-utilization.json').write_text(json.dumps(report))
            snapshots()
            if args.queue_diagnostics:
                fixture = root / 'target/debug/examples/circuit_queue_fixture'
                processes.append(subprocess.Popen([str(fixture), str(path/'analytics.sock')], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL))
                deadline = time.monotonic()+2
                while not (path/'analytics.sock').exists():
                    if time.monotonic() > deadline:
                        raise RuntimeError('Synthetic analytics fixture did not start')
                    time.sleep(.02)
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
            if args.screen == 1:
                if args.traffic_source != 'combined':
                    for _ in range(['actual','mapped','unmapped'].index(args.traffic_source)+1):
                        key('v');time.sleep(.05)
                time.sleep(.2)
            if args.subscriber_stats or args.queue_diagnostics:
                key('slash');time.sleep(.1)
                for char in 'subscriber 00':
                    key('space' if char==' ' else char);time.sleep(.03)
                key('Return');time.sleep(.2)
                key('e'); time.sleep(.3)
                if args.queue_diagnostics:
                    key('d'); time.sleep(.8)
                xt.XTestFakeMotionEvent.argtypes=[C.c_void_p,C.c_int,C.c_int,C.c_int,C.c_ulong]
                xt.XTestFakeButtonEvent.argtypes=[C.c_void_p,C.c_uint,C.c_int,C.c_ulong]
                xt.XTestFakeMotionEvent(d,-1,1000,350,0)
                xt.XTestFakeButtonEvent(d,1,1,0);xt.XTestFakeButtonEvent(d,1,0,0);x.XFlush(d)
                time.sleep(.2);key('Home')
                for _ in range(25 if args.queue_diagnostics else 15):
                    key('Down');time.sleep(.03)
                time.sleep(.4)
            if args.worker_health:
                key('e'); key('Tab')
                for _ in range(27): key('Down')
                time.sleep(.2)
            if args.worker_phases:
                key('e'); key('Tab')
                for _ in range(12): key('Down')
                time.sleep(.2)
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
