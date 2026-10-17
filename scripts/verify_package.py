#!/usr/bin/env python3
"""Verify a native CI package without extracting untrusted paths or launching a GUI."""
import argparse
import hashlib
import platform
import plistlib
import subprocess
import tarfile
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('directory', type=Path)
args = parser.parse_args()
version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
packages = [p for p in args.directory.iterdir()
            if p.is_file() and p.name.endswith(('.dmg', '.zip', '.tar.gz'))]
if len(packages) != 1:
    raise SystemExit('Expected exactly one package in the output directory')
package = packages[0]
with package.open('rb') as stream:
    digest = hashlib.file_digest(stream, 'sha256').hexdigest()
expected = f'{digest}  {package.name}\n'
if (args.directory / 'SHA256SUMS').read_text(encoding='utf-8') != expected:
    raise SystemExit('Package checksum mismatch')

prefix = f'yo3mfs-{version}/'
if package.suffix == '.dmg':
    if platform.system() != 'Darwin':
        raise SystemExit('DMG validation requires macOS')
    bundle = args.directory / 'yo3mfs.app'
    info = plistlib.loads((bundle / 'Contents/Info.plist').read_bytes())
    assert info['CFBundleShortVersionString'] == version
    assert info['CFBundleIconFile'] == 'yo3mfs.icns'
    assert (bundle / 'Contents/Resources/yo3mfs.icns').read_bytes() == (root / 'assets/branding/yo3mfs.icns').read_bytes()
    assert (bundle / 'Contents/Resources/NotoSansSymbols2-OFL.txt').is_file()
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(bundle)], check=True)
    subprocess.run(['hdiutil', 'verify', str(package)], check=True)
elif package.suffix == '.zip':
    with zipfile.ZipFile(package) as archive:
        assert archive.testzip() is None
        for name in ['yo3mfs.exe', 'yo3mfs.ico', 'README.md', 'NotoSansSymbols2-OFL.txt']:
            assert archive.getinfo(prefix + name).file_size > 0, name
else:
    with tarfile.open(package, 'r:gz') as archive:
        binary = archive.getmember(prefix + 'yo3mfs')
        assert binary.isfile() and binary.mode & 0o111, 'Linux binary is not executable'
        for name in ['README.md', 'NotoSansSymbols2-OFL.txt',
                     'share/applications/com.yo3mfs.viewer.desktop',
                     'share/icons/hicolor/512x512/apps/com.yo3mfs.viewer.png']:
            member = archive.getmember(prefix + name)
            assert member.isfile() and member.size > 0, name
print(f'Verified {package.name}: SHA-256 {digest}')
