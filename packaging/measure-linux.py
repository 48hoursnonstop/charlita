#!/usr/bin/env python3
"""Measure an isolated, idle native editor with repeatable PNG scenes."""
import argparse
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser()
parser.add_argument('--binary', type=Path, default=Path('build-release/charlita'))
parser.add_argument('--seconds', type=int, default=30)
parser.add_argument('--platform', choices=['offscreen', 'wayland', 'xcb'], default='offscreen')
parser.add_argument('--canvas', choices=['automatic', 'fixed'], default='automatic')
parser.add_argument('--output', type=Path, default=Path('dist/measurements.json'))
args = parser.parse_args()
if not Path('/proc/self/stat').is_file() or args.seconds < 5:
    raise SystemExit('Use Linux and a sampling period of at least five seconds')
binary = args.binary.resolve()
subprocess.run(['cargo', 'build', '--locked', '--example', 'scene'], check=True)
ticks = os.sysconf('SC_CLK_TCK')

def sample(pid):
    root = Path('/proc') / str(pid)
    fields = (root / 'stat').read_text().rsplit(')', 1)[1].split()
    memory = {}
    for line in (root / 'smaps_rollup').read_text().splitlines():
        if line.startswith(('Rss:', 'Pss:')):
            key, value, _ = line.split()
            memory[key[:-1]] = int(value) / 1024
    return int(fields[11]) + int(fields[12]), memory

results = []
with tempfile.TemporaryDirectory(prefix='charlita-measure-') as directory:
    for count in (8, 32, 128):
        data = Path(directory) / str(count)
        fixture = ['target/debug/examples/scene', str(data), str(count)]
        if args.canvas == 'automatic':
            fixture.append('--auto')
        subprocess.run(fixture, check=True,
                       stdout=subprocess.DEVNULL)
        env = {**os.environ, 'QT_QPA_PLATFORM': args.platform, 'QT_SCALE_FACTOR': '1'}
        if args.platform == 'offscreen':
            env['QT_QUICK_BACKEND'] = 'software'
        with (Path(directory) / f'{count}.log').open('w') as log:
            process = subprocess.Popen([str(binary), '--data-dir', str(data), '--port', '39469'],
                                       env=env, stdout=log, stderr=log)
            try:
                time.sleep(3)
                if process.poll() is not None:
                    raise SystemExit(f'Native editor exited before sampling {count} guests')
                start_ticks, initial = sample(process.pid)
                start = time.monotonic()
                memory = [initial]
                for _ in range(args.seconds):
                    time.sleep(1)
                    end_ticks, current = sample(process.pid)
                    memory.append(current)
                elapsed = time.monotonic() - start
                result = {
                    'guests': count, 'seconds': round(elapsed, 2),
                    'cpu_percent_one_core': round(100 * (end_ticks-start_ticks) / ticks / elapsed, 3),
                    'rss_mib_mean': round(statistics.mean(m['Rss'] for m in memory), 2),
                    'rss_mib_max_sample': round(max(m['Rss'] for m in memory), 2),
                    'pss_mib_mean': round(statistics.mean(m['Pss'] for m in memory), 2),
                }
                results.append(result)
                print(json.dumps(result), flush=True)
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
report = {'platform': args.platform, 'canvas': args.canvas, 'binary': str(binary),
          'scenario': 'Idle editor, grid, distinct 128x128 PNGs, Discord and update checks off; no browser or OBS included',
          'results': results}
args.output.parent.mkdir(parents=True, exist_ok=True)
args.output.write_text(json.dumps(report, indent=2) + '\n')
