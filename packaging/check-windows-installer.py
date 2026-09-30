#!/usr/bin/env python3
"""Exercise installation, native startup and data retention on a disposable runner."""
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile

if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
    raise SystemExit('Use a disposable Windows Actions runner for this installer check')
import winreg

key = r'Software\Microsoft\Windows\CurrentVersion\Uninstall\Charlita'
try:
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key):
        raise SystemExit('A Charlita installation already exists; refusing to overwrite it')
except FileNotFoundError:
    pass

installer = next(Path(sys.argv[1]).resolve().glob('*-windows-x86_64-setup.exe'))
with tempfile.TemporaryDirectory(prefix='charlita-install-') as directory:
    install = Path(directory) / 'installed'
    # NSIS requires /D to be last and its directory to remain unquoted.
    subprocess.run(f'"{installer}" /S /D={install}', check=True, timeout=60)
    subprocess.run([sys.executable, str(Path(__file__).with_name('check-startup.py')),
                    str(install)], check=True, timeout=45)
    retained = install / 'bin' / 'data' / 'retained.txt'
    retained.parent.mkdir(parents=True)
    retained.write_text('Portable user data must survive uninstall', encoding='utf-8')
    # Run the copy directly so the wait includes the actual uninstall.
    # https://nsis.sourceforge.io/Docs/Chapter3.html#3.2.2
    uninstaller = Path(directory) / 'uninstall-test.exe'
    shutil.copy2(install / 'Uninstall.exe', uninstaller)
    subprocess.run(f'"{uninstaller}" /S _?={install}', check=True, timeout=60)
    if (install / 'bin' / 'charlita.exe').exists() or not retained.is_file():
        raise SystemExit('Uninstall must remove the executable and preserve user data')
    if retained.read_text(encoding='utf-8') != 'Portable user data must survive uninstall':
        raise SystemExit('Uninstall changed user data')
print('Windows installer opened its native window and preserved user data on uninstall')
