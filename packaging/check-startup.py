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
with tempfile.TemporaryDirectory(prefix='charlita-startup-') as data:
    subprocess.run([str(executable), '--data-dir', data, '--port', '39468',
                    '--screenshot', str(screenshot)], check=True, timeout=30,
                   env={**os.environ, 'QT_QPA_PLATFORM': 'offscreen',
                        'QT_QUICK_BACKEND': 'software'})
if not screenshot.is_file() or screenshot.read_bytes()[:8] != b'\x89PNG\r\n\x1a\n':
    raise SystemExit('Deployed Charlita did not produce a window capture')
print('Deployed native window opened and rendered:', screenshot)
