#!/usr/bin/env python3
"""Explicit optional download + hash/geometry check; the desktop app stays offline."""
import argparse
import hashlib
import json
import subprocess
import tempfile
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--download', action='store_true')
args = parser.parse_args()
fixtures = json.loads((root / 'tests/fixtures/external.json').read_text())
for fixture in fixtures:
    path = root / 'target' / fixture['name']
    if not path.exists() and args.download:
        path.parent.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(dir=path.parent) as temp:
            download = Path(temp) / 'download'
            subprocess.run(['curl', '--fail', '--location', '--silent', '--show-error',
                            '--max-filesize', str(64 * 1024 * 1024),
                            fixture['url'], '-o', str(download)], check=True)
            expected = fixture.get('download_sha256', fixture['sha256'])
            if hashlib.sha256(download.read_bytes()).hexdigest() != expected:
                raise SystemExit(f'Download hash mismatch: {fixture["name"]}')
            if 'archive_member' in fixture:
                with zipfile.ZipFile(download) as archive:
                    member = archive.getinfo(fixture['archive_member'])
                    if member.file_size > 128 * 1024 * 1024:
                        raise SystemExit('Reference member exceeds size limit')
                    data = archive.read(member)
            else:
                data = download.read_bytes()
            if hashlib.sha256(data).hexdigest() != fixture['sha256']:
                raise SystemExit(f'Member hash mismatch: {fixture["name"]}')
            path.write_bytes(data)
    if not path.exists():
        raise SystemExit('Missing reference model; pass --download to fetch pinned public fixtures.')
    if hashlib.sha256(path.read_bytes()).hexdigest() != fixture['sha256']:
        raise SystemExit(f'Hash mismatch: {path}')
subprocess.run(['cargo', 'build', '--locked', '--release', '--no-default-features',
                '--example', 'inspect'], cwd=root, check=True)
binary = root / 'target/release/examples/inspect'
if not binary.exists():
    binary = binary.with_suffix('.exe')
for fixture in fixtures:
    path = root / 'target' / fixture['name']
    actual = json.loads(subprocess.check_output([str(binary), str(path)], text=True))
    assert actual['instances'] == fixture['instances'], actual
    assert actual['triangles'] == fixture['triangles'], actual
    for key in ['palette', 'instance_filaments', 'painted_facets', 'rendered_triangles']:
        assert actual[key] == fixture[key], (fixture['name'], key, actual[key])
    assert all(abs(a-b) < 0.001 for a,b in zip(actual['size_mm'], fixture['size_mm'])), actual
    assert hashlib.sha256(path.read_bytes()).hexdigest() == fixture['sha256']
    print(json.dumps({'fixture': fixture['name'], 'result': actual}, ensure_ascii=False))
