#!/usr/bin/env python3
"""Package a built binary without publishing or choosing a project license."""
import argparse
import hashlib
import json
import platform
import plistlib
import shutil
import subprocess
import tarfile
import tomllib
import zipfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
version = tomllib.loads((root / 'Cargo.toml').read_text())['package']['version']
parser = argparse.ArgumentParser()
parser.add_argument('--profile', choices=['debug', 'release'], default='release')
parser.add_argument('--target', default='')
parser.add_argument('--dmg', action='store_true')
parser.add_argument('--output-dir', type=Path, help='Separate output directory; defaults to version/platform/profile in dist')
args = parser.parse_args()
system = platform.system()
arch = args.target or platform.machine()
binary = root / 'target' / args.target / args.profile / ('yo3mfs.exe' if system == 'Windows' else 'yo3mfs')
if not binary.is_file():
    raise SystemExit(f'Build first: missing {binary}')
platform_name = 'macos' if system == 'Darwin' else system.lower()
out = args.output_dir or root / 'dist' / f'{version}-{platform_name}-{arch}-{args.profile}'
out.mkdir(parents=True, exist_ok=True)
if system == 'Darwin':
    bundle = out / 'yo3mfs.app'
    contents = bundle / 'Contents'
    (contents / 'MacOS').mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, contents / 'MacOS' / 'yo3mfs')
    (contents / 'Resources').mkdir(exist_ok=True)
    shutil.copy2(root / 'assets/fonts/OFL.txt', contents / 'Resources' / 'NotoSansSymbols2-OFL.txt')
    shutil.copy2(root / 'assets/branding/yo3mfs.icns', contents / 'Resources' / 'yo3mfs.icns')
    shutil.copy2(root / 'docs/PACKAGE.md', contents / 'Resources' / 'README.md')
    with (contents / 'Info.plist').open('wb') as f:
        plistlib.dump({
            'CFBundleName': 'YO3MFS', 'CFBundleDisplayName': 'YO3MFS',
            'CFBundleIdentifier': 'com.yo3mfs.viewer', 'CFBundleExecutable': 'yo3mfs',
            'CFBundlePackageType': 'APPL', 'CFBundleShortVersionString': version,
            'CFBundleVersion': version, 'NSHighResolutionCapable': True,
            'CFBundleIconFile': 'yo3mfs.icns',
            'LSMinimumSystemVersion': '12.0',
        }, f)
    subprocess.run(['codesign', '--force', '--sign', '-', '--timestamp=none', str(bundle)], check=True)
    if args.dmg:
        dmg = out / f'yo3mfs-{version}-{arch}.dmg'
        subprocess.run(['hdiutil', 'create', '-volname', 'yo3mfs', '-srcfolder', str(bundle),
                        '-ov', '-format', 'UDZO', str(dmg)], check=True)
    artifact = dmg if args.dmg else bundle
else:
    folder = out / f'yo3mfs-{version}'
    folder.mkdir(exist_ok=True)
    shutil.copy2(binary, folder / binary.name)
    shutil.copy2(root / 'docs/PACKAGE.md', folder / 'README.md')
    shutil.copy2(root / 'assets/fonts/OFL.txt', folder / 'NotoSansSymbols2-OFL.txt')
    if system == 'Windows':
        shutil.copy2(root / 'assets/branding/yo3mfs.ico', folder / 'yo3mfs.ico')
        artifact = out / f'yo3mfs-{version}-{arch}.zip'
        with zipfile.ZipFile(artifact, 'w', zipfile.ZIP_DEFLATED) as z:
            for f in folder.iterdir():
                z.write(f, f'{folder.name}/{f.name}')
    else:
        applications = folder / 'share' / 'applications'
        icons = folder / 'share' / 'icons' / 'hicolor' / '512x512' / 'apps'
        applications.mkdir(parents=True, exist_ok=True)
        icons.mkdir(parents=True, exist_ok=True)
        shutil.copy2(root / 'assets/linux/com.yo3mfs.viewer.desktop', applications)
        shutil.copy2(root / 'assets/branding/icons/512.png', icons / 'com.yo3mfs.viewer.png')
        artifact = out / f'yo3mfs-{version}-{arch}.tar.gz'
        with tarfile.open(artifact, 'w:gz') as tar:
            tar.add(folder, arcname=folder.name)
if artifact.is_file():
    with artifact.open('rb') as stream:
        checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
    (out / 'SHA256SUMS').write_text(f'{checksum}  {artifact.name}\n', encoding='utf-8')
print(json.dumps({'artifact': str(artifact), 'signed_for_distribution': False}))
