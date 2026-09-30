#!/usr/bin/env python3
"""Create native distributions from an already deployed CMake install tree."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
VERSION = tomllib.loads((ROOT / 'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
APP_ID = 'io.github.48hoursnonstop.charlita'

def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)

def download(url, digest, path):
    if not path.exists():
        urllib.request.urlretrieve(url, path)
    with path.open('rb') as source:
        actual = hashlib.file_digest(source, 'sha256').hexdigest()
    if actual != digest:
        path.unlink()
        raise SystemExit('Packaging tool checksum mismatch: ' + url)
    path.chmod(0o755)
    return path

parser = argparse.ArgumentParser()
parser.add_argument('--stage', type=Path, required=True)
parser.add_argument('--qt-sources', type=Path, required=True)
parser.add_argument('--output', type=Path, default=ROOT / 'dist' / 'packages')
args = parser.parse_args()
stage, output = args.stage.resolve(), args.output.resolve()
core = stage / 'bin' / 'Qt6Core.dll' if os.name == 'nt' else stage / 'lib' / 'libQt6Core.so.6'
if not core.is_file():
    raise SystemExit('Qt runtime libraries are missing from the deployed tree: ' + str(core))
output.mkdir(parents=True, exist_ok=True)
for name in ('LICENSE', 'THIRD-PARTY-NOTICES.md'):
    shutil.copy2(ROOT / name, stage / name)
licenses = stage / 'licenses'
licenses.mkdir(exist_ok=True)
shutil.copy2(ROOT / 'ui' / 'fonts' / 'OFL.txt', licenses / 'Inter-OFL.txt')
run('cargo', 'bundle-licenses', '--format', 'json', '--output', licenses / 'Rust.json', cwd=ROOT)
# cargo-bundle-licenses scans every target. Keep the dependency graph that is
# actually linked into this distribution, including build/proc-macro notices.
host = next(line.removeprefix('host: ') for line in subprocess.check_output(['rustc', '-vV'], text=True, encoding='utf-8').splitlines() if line.startswith('host: '))
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', host], cwd=ROOT, text=True, encoding='utf-8'))
nodes = {node['id']: node for node in metadata['resolve']['nodes']}
packages = {package['id']: package for package in metadata['packages']}
pending, reached = [metadata['resolve']['root']], set()
while pending:
    package_id = pending.pop()
    if package_id in reached:
        continue
    reached.add(package_id)
    pending.extend(dep['pkg'] for dep in nodes[package_id]['deps'] if any(kind['kind'] != 'dev' for kind in dep['dep_kinds']))
linked = {(packages[p]['name'], packages[p]['version']) for p in reached}
bundle = json.loads((licenses / 'Rust.json').read_text(encoding='utf-8'))
bundle['third_party_libraries'] = [package for package in bundle['third_party_libraries'] if (package['package_name'], package['package_version']) in linked]
missing = [package['package_name'] for package in bundle['third_party_libraries'] if any(license['text'] == 'NOT FOUND' for license in package['licenses'])]
if missing:
    raise SystemExit('Missing license notices for linked dependencies: ' + ', '.join(missing))
(licenses / 'Rust.json').write_text(json.dumps(bundle, indent=2), encoding='utf-8')
qt_licenses = licenses / 'Qt'
count = 0
for source in args.qt_sources.rglob('*'):
    if source.is_file() and (source.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE', 'QT_ATTRIBUTION')) or 'LICENSES' in source.parts):
        dest = qt_licenses / source.relative_to(args.qt_sources)
        dest.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(source, dest)
        count += 1
if not count:
    raise SystemExit('No Qt source license notices found; install the matching Qt sources')
shutil.copytree(ROOT / 'tools' / 'licenses', licenses / 'FFmpeg', dirs_exist_ok=True)
for tool in ('ffmpeg', 'ffprobe'):
    filename = tool + ('.exe' if os.name == 'nt' else '')
    shutil.copy2(ROOT / 'tools' / filename, stage / 'bin' / filename)

prefix = f'charlita-{VERSION}'
if os.name == 'nt':
    compiler = shutil.which('makensis') or str(Path(os.environ.get('ProgramFiles(x86)', 'C:/Program Files (x86)')) / 'NSIS' / 'makensis.exe')
    run(compiler, f'/DSOURCE={ROOT}', f'/DSTAGE={stage}', f'/DVERSION={VERSION}', f'/DOUTPUT={output / (prefix + "-windows-x86_64-setup.exe")}', ROOT / 'packaging' / 'windows.nsi')
    # A portable flag sits beside the real executable. No account credentials are bundled.
    (stage / 'bin' / 'portable.flag').touch()
    (stage / 'Charlita.cmd').write_text('@echo off\r\nstart "" "%~dp0bin\\charlita.exe" %*\r\n', encoding='utf-8')
    shutil.make_archive(str(output / (prefix + '-windows-x86_64-portable')), 'zip', stage)
    (stage / 'bin' / 'portable.flag').unlink()
else:
    with tempfile.TemporaryDirectory(prefix='charlita-package-') as directory:
        work = Path(directory)
        appdir = work / 'Charlita.AppDir'
        shutil.copytree(stage, appdir / 'usr', symlinks=True)
        shutil.copy2(ROOT / 'packaging' / 'AppRun', appdir / 'AppRun')
        (appdir / 'AppRun').chmod(0o755)
        shutil.copy2(ROOT / 'packaging' / (APP_ID + '.desktop'), appdir / (APP_ID + '.desktop'))
        shutil.copy2(ROOT / 'ui' / 'logo.svg', appdir / (APP_ID + '.svg'))
        cache = ROOT / 'dist' / 'cache'
        cache.mkdir(parents=True, exist_ok=True)
        tool = download('https://github.com/AppImage/appimagetool/releases/download/1.9.1/appimagetool-x86_64.AppImage', 'ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0', cache / 'appimagetool-1.9.1.AppImage')
        run(tool, '--appimage-extract', cwd=work, stdout=subprocess.DEVNULL)
        # Reuse the runtime inside the verified tool instead of downloading a
        # changing runtime automatically during packaging.
        offset = int(subprocess.check_output([str(tool), '--appimage-offset']).strip())
        runtime = work / 'runtime-x86_64'
        with tool.open('rb') as source:
            runtime.write_bytes(source.read(offset))
        run(work / 'squashfs-root' / 'AppRun', '--runtime-file', runtime, '--no-appstream', appdir, output / (prefix + '-linux-x86_64.AppImage'), env={**os.environ, 'ARCH': 'x86_64'})
        deb = work / 'deb'
        shutil.copytree(stage, deb / 'opt' / 'charlita', symlinks=True)
        (deb / 'usr' / 'bin').mkdir(parents=True)
        (deb / 'usr' / 'bin' / 'charlita').symlink_to('/opt/charlita/bin/charlita')
        applications = deb / 'usr' / 'share' / 'applications'
        applications.mkdir(parents=True)
        shutil.copy2(ROOT / 'packaging' / (APP_ID + '.desktop'), applications)
        icons = deb / 'usr' / 'share' / 'icons' / 'hicolor' / 'scalable' / 'apps'
        icons.mkdir(parents=True)
        shutil.copy2(ROOT / 'ui' / 'logo.svg', icons / (APP_ID + '.svg'))
        (deb / 'DEBIAN').mkdir()
        (deb / 'DEBIAN' / 'control').write_text(f'Package: charlita\nVersion: {VERSION}\nArchitecture: amd64\nMaintainer: Charlita contributors <opensource@users.noreply.github.com>\nSection: video\nPriority: optional\nDepends: libc6 (>= 2.35), libx11-6, libgl1, libopengl0, libegl1, libdbus-1-3, libxkbcommon0\nDescription: Reactive Discord guest overlays for OBS and Streamlabs\n Native Qt editor and local transparent overlays.\n', encoding='utf-8')
        run('dpkg-deb', '--root-owner-group', '--build', deb, output / (prefix + '-linux-x86_64.deb'))
        (stage / 'bin' / 'portable.flag').touch()
        launcher = stage / 'charlita'
        launcher.write_text('#!/bin/sh\ncharlita_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)\nexec "$charlita_dir/bin/charlita" "$@"\n')
        launcher.chmod(0o755)
        with tarfile.open(output / (prefix + '-linux-x86_64-portable.tar.gz'), 'w:gz') as archive:
            archive.add(stage, arcname='Charlita')
        (stage / 'bin' / 'portable.flag').unlink()

with (output / 'SHA256SUMS').open('w', encoding='utf-8') as sums:
    for package in sorted(output.iterdir()):
        if package.is_file() and package.name != 'SHA256SUMS':
            with package.open('rb') as source:
                digest = hashlib.file_digest(source, 'sha256').hexdigest()
            sums.write(f'{digest}  {package.name}\n')
print('Native distributions:', output)
