#!/usr/bin/env python3
"""Explicit optional download + hash/geometry check; the desktop app stays offline."""
import argparse
import hashlib
import json
import subprocess
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
        subprocess.run(['curl', '--fail', '--location', '--silent', '--show-error',
                        fixture['url'], '-o', str(path)], check=True)
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
    assert all(abs(a-b) < 0.001 for a,b in zip(actual['size_mm'], fixture['size_mm'])), actual
    assert hashlib.sha256(path.read_bytes()).hexdigest() == fixture['sha256']
    print(json.dumps({'fixture': fixture['name'], 'result': actual}, ensure_ascii=False))
