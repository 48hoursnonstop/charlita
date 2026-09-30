#!/usr/bin/env python3
"""Check the deployed executable, including Qt plugins, on either platform."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

stage = Path(sys.argv[1]).resolve()
executable = stage / 'bin' / ('charlita.exe' if os.name == 'nt' else 'charlita')
screenshot = stage.parent / 'deployed.png'
screenshot.unlink(missing_ok=True)
env = {**os.environ, 'QT_QPA_PLATFORM': 'offscreen', 'QT_QUICK_BACKEND': 'software'}
for name in ('LD_LIBRARY_PATH', 'QT_PLUGIN_PATH', 'QT_QPA_PLATFORM_PLUGIN_PATH',
             'QML_IMPORT_PATH', 'QML2_IMPORT_PATH'):
    env.pop(name, None)
if qt_root := os.environ.get('QT_ROOT_DIR'):
    sdk = Path(qt_root).resolve()
    env['PATH'] = os.pathsep.join(entry for entry in env.get('PATH', '').split(os.pathsep)
                                if entry and not Path(entry).resolve().is_relative_to(sdk))
with tempfile.TemporaryDirectory(prefix='charlita-startup-') as data:
    subprocess.run([str(executable), '--data-dir', data, '--port', '39468',
                    '--screenshot', str(screenshot)], check=True, timeout=30,
                   env=env)
if not screenshot.is_file() or screenshot.read_bytes()[:8] != b'\x89PNG\r\n\x1a\n':
    raise SystemExit('Deployed Charlita did not produce a window capture')
print('Deployed native window opened and rendered:', screenshot)
