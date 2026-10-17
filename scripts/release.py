#!/usr/bin/env python3
"""Validate a version tag, collect all native packages, and publish them together."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tomllib


ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    'yo3mfs-macos-arm64': 'aarch64-apple-darwin.dmg',
    'yo3mfs-macos-x64': 'x86_64-apple-darwin.dmg',
    'yo3mfs-windows-x64': 'x86_64-pc-windows-msvc.zip',
    'yo3mfs-linux-x64': 'x86_64-unknown-linux-gnu.tar.gz',
}


def check_tag(root, tag):
    version = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    if not re.fullmatch(r'v\d+\.\d+\.\d+', tag) or tag != f'v{version}':
        raise ValueError(f'Release tag must be v{version}, received {tag!r}')
    lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
    versions = [p['version'] for p in lock['package'] if p['name'] == 'yo3mfs']
    if versions != [version]:
        raise ValueError('Cargo.lock version does not match Cargo.toml')
    notes = root / 'docs' / 'releases' / f'{tag}.md'
    if not notes.is_file() or not notes.read_text(encoding='utf-8').strip():
        raise ValueError(f'Missing release notes: {notes}')
    return version, notes


def prepare(artifacts, output, version):
    if set(p.name for p in artifacts.iterdir()) != set(TARGETS):
        raise ValueError('Expected exactly the four native platform artifacts')
    packages = []
    checksums = []
    for artifact, suffix in TARGETS.items():
        name = f'yo3mfs-{version}-{suffix}'
        directory = artifacts / artifact
        if set(p.name for p in directory.iterdir()) != {name, 'SHA256SUMS'}:
            raise ValueError(f'Unexpected package contents for {artifact}')
        package = directory / name
        if package.is_symlink() or not package.is_file() or package.stat().st_size == 0:
            raise ValueError(f'Invalid package: {name}')
        with package.open('rb') as stream:
            digest = hashlib.file_digest(stream, 'sha256').hexdigest()
        checksum = f'{digest}  {name}\n'
        if (directory / 'SHA256SUMS').read_text(encoding='utf-8') != checksum:
            raise ValueError(f'Checksum mismatch: {name}')
        packages.append(package)
        checksums.append(checksum)

    # Validate every platform before writing anything destined for a Release.
    expected = {p.name for p in packages} | {'SHA256SUMS'}
    if output.exists() and not set(p.name for p in output.iterdir()) <= expected:
        raise ValueError('Release output contains unexpected files; use a clean directory')
    output.mkdir(parents=True, exist_ok=True)
    for package in packages:
        shutil.copyfile(package, output / package.name)
    (output / 'SHA256SUMS').write_text(''.join(sorted(checksums)), encoding='utf-8')
    return [output / p.name for p in packages] + [output / 'SHA256SUMS']


def publish(repository, tag, notes, assets):
    # Listing fails closed on auth/network errors; only an absent tag creates a draft.
    pages = json.loads(subprocess.check_output(
        ['gh', 'api', f'repos/{repository}/releases', '--paginate', '--slurp'], text=True))
    existing = next((release for page in pages for release in page
                     if release['tag_name'] == tag), None)
    if existing and not existing['draft']:
        raise ValueError(f'{tag} is already published; published assets are never overwritten')
    base = ['gh', 'release']
    repo = ['--repo', repository]
    if not existing:
        subprocess.run(base + ['create', tag, '--verify-tag', '--draft', '--title',
                              f'YO3MFS {tag}', '--notes-file', str(notes)] + repo, check=True)
    # Only drafts can be resumed/replaced. Publication happens after every upload succeeds.
    subprocess.run(base + ['upload', tag, *map(str, assets), '--clobber'] + repo, check=True)
    subprocess.run(base + ['edit', tag, '--draft=false', '--latest', '--title',
                          f'YO3MFS {tag}', '--notes-file', str(notes)] + repo, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['check', 'prepare', 'publish'])
    parser.add_argument('--artifacts', type=Path, default=ROOT / 'dist/artifacts')
    parser.add_argument('--output', type=Path, default=ROOT / 'dist/release')
    args = parser.parse_args()
    tag = os.environ.get('RELEASE_TAG', '')
    version, notes = check_tag(ROOT, tag)
    if args.command == 'check':
        print(f'Verified release tag {tag}, Cargo versions, and release notes')
        return
    assets = prepare(args.artifacts, args.output, version)
    print(f'Verified four packages and combined SHA256SUMS for {tag}', flush=True)
    if args.command == 'publish':
        publish(os.environ['GITHUB_REPOSITORY'], tag, notes, assets)


if __name__ == '__main__':
    main()
