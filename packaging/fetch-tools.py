#!/usr/bin/env python3
"""Fetch the pinned FFmpeg tools; verify before extracting anything."""
import hashlib
import os
from pathlib import Path
import shutil
import sys
import tarfile
import urllib.request
import zipfile

TAG = 'autobuild-2026-08-31-13-27'  # Monthly build retained upstream for two years.
FILES = {
    'linux': ('ffmpeg-n8.1.2-50-g1a748fe2cd-linux64-lgpl-8.1.tar.xz', '7d6d93e9c39e0e461feb13c118e91e4eec2515e4da3a01d4ad6790996731bbee'),
    'windows': ('ffmpeg-n8.1.2-50-g1a748fe2cd-win64-lgpl-8.1.zip', 'f6274bbd9c247f9e90c1bbed066b03ed4a3907cece2fb91be6dd352393936365'),
}
platform = sys.argv[1] if len(sys.argv) > 1 else ('windows' if os.name == 'nt' else 'linux')
name, digest = FILES[platform]
root = Path(__file__).resolve().parents[1]
cache = root / 'dist' / 'cache'
cache.mkdir(parents=True, exist_ok=True)
archive = cache / name
if not archive.exists():
    print('Downloading', name, flush=True)
    urllib.request.urlretrieve(f'https://github.com/BtbN/FFmpeg-Builds/releases/download/{TAG}/{name}', archive)
if hashlib.file_digest(archive.open('rb'), 'sha256').hexdigest() != digest:
    archive.unlink()
    raise SystemExit('FFmpeg checksum mismatch; download discarded')
out = root / 'tools'
out.mkdir(exist_ok=True)
licenses = out / 'licenses'
licenses.mkdir(exist_ok=True)
ext = '.exe' if platform == 'windows' else ''
with (zipfile.ZipFile(archive) if platform == 'windows' else tarfile.open(archive)) as bundle:
    names = bundle.namelist() if platform == 'windows' else bundle.getnames()
    for tool in ('ffmpeg', 'ffprobe'):
        filename = tool + ext
        member = next(n for n in names if n.endswith('/bin/' + filename))
        with (bundle.open(member) if platform == 'windows' else bundle.extractfile(member)) as source, (out / filename).open('wb') as dest:
            shutil.copyfileobj(source, dest)
        (out / filename).chmod(0o755)
    for member in names:
        basename = Path(member).name
        if basename.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
            source = bundle.open(member) if platform == 'windows' else bundle.extractfile(member)
            if source:
                with source, (licenses / basename).open('wb') as dest:
                    shutil.copyfileobj(source, dest)
(licenses / 'FFmpeg-build.txt').write_text(f'FFmpeg LGPL tool build: {TAG}\nArchive: {name}\nSHA256: {digest}\nBuild scripts and dependency sources: https://github.com/BtbN/FFmpeg-Builds/tree/{TAG}\nFFmpeg source revision: 1a748fe2cd (release/8.1)\nhttps://git.ffmpeg.org/ffmpeg.git\n', encoding='utf-8')
print('Verified FFmpeg and ffprobe:', out)
