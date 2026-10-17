#!/usr/bin/env python3
"""Sign and notarize a macOS release using GitRonimo-compatible CI secrets."""
import argparse
import base64
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import time
import tomllib


ROOT = Path(__file__).resolve().parent.parent
SECRET_NAMES = (
    'DEVELOPER_ID_APPLICATION', 'MACOS_CERTIFICATE_BASE64',
    'MACOS_CERTIFICATE_PASSWORD', 'KEYCHAIN_PASSWORD',
    'APPLE_API_KEY_BASE64', 'APPLE_API_KEY_ID', 'APPLE_API_ISSUER',
)


def configuration(environ):
    missing = [name for name in SECRET_NAMES if not environ.get(name)]
    if missing:
        raise ValueError('Missing macOS release secrets: ' + ', '.join(missing))
    config = {name: environ[name] for name in SECRET_NAMES}
    if not config['DEVELOPER_ID_APPLICATION'].startswith('Developer ID Application:'):
        raise ValueError('DEVELOPER_ID_APPLICATION must be a Developer ID Application identity')
    for name in ['MACOS_CERTIFICATE_BASE64', 'APPLE_API_KEY_BASE64']:
        try:
            if not base64.b64decode(config[name], validate=True):
                raise ValueError()
        except ValueError:
            raise ValueError(f'{name} must contain valid, nonempty base64') from None
    return config


def run(command, label, config, *, check=True):
    # security/import receives passwords as arguments. Never expose a command or
    # CalledProcessError in CI logs, including failures during keychain cleanup.
    result = subprocess.run(command, capture_output=True, text=True)
    if check and result.returncode:
        diagnostic = result.stderr or result.stdout
        for value in config.values():
            diagnostic = diagnostic.replace(value, '[redacted]')
        raise RuntimeError(f'{label} failed (exit {result.returncode}): {diagnostic[-2000:]}')
    return result


def cleanup(directory):
    keychain = directory / 'release.keychain-db'
    deletion_failed = False
    if keychain.exists():
        result = subprocess.run(['security', 'delete-keychain', str(keychain)],
                                capture_output=True, text=True)
        deletion_failed = result.returncode != 0
    if directory.exists():
        shutil.rmtree(directory)
    if deletion_failed:
        raise RuntimeError('Keychain cleanup command failed; temporary credential files were removed')


@contextmanager
def signing_credentials(config, directory):
    # This directory belongs only to this job. Refuse to reuse leftover material.
    directory.mkdir(mode=0o700)
    keychain = directory / 'release.keychain-db'
    certificate = directory / 'certificate.p12'
    api_key = directory / 'api-key.p8'
    try:
        for path, name in [(certificate, 'MACOS_CERTIFICATE_BASE64'), (api_key, 'APPLE_API_KEY_BASE64')]:
            path.write_bytes(base64.b64decode(config[name], validate=True))
            path.chmod(0o600)
        password = config['KEYCHAIN_PASSWORD']
        run(['security', 'create-keychain', '-p', password, str(keychain)], 'Create keychain', config)
        run(['security', 'set-keychain-settings', '-lut', '7200', str(keychain)], 'Configure keychain', config)
        run(['security', 'unlock-keychain', '-p', password, str(keychain)], 'Unlock keychain', config)
        run(['security', 'import', str(certificate), '-k', str(keychain), '-P',
             config['MACOS_CERTIFICATE_PASSWORD'], '-T', '/usr/bin/codesign'], 'Import certificate', config)
        run(['security', 'set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:',
             '-s', '-k', password, str(keychain)], 'Allow codesign access', config)
        certificate.unlink()
        yield keychain, ['--key', str(api_key), '--key-id', config['APPLE_API_KEY_ID'],
                        '--issuer', config['APPLE_API_ISSUER']]
    finally:
        cleanup(directory)


def notarize(dmg, authentication, config):
    result = run(['xcrun', 'notarytool', 'submit', str(dmg), *authentication,
                  '--output-format', 'json'], 'Submit notarization', config)
    submission = json.loads(result.stdout)['id']
    print(f'Apple notarization submission: {submission}', flush=True)
    # Poll the same submission on timeouts. Never upload duplicates while waiting.
    for attempt in range(3):
        result = run(['xcrun', 'notarytool', 'wait', submission, *authentication,
                      '--timeout', '900', '--output-format', 'json'],
                     'Wait for notarization', config, check=False)
        try:
            status = json.loads(result.stdout).get('status')
        except json.JSONDecodeError:
            status = None
        if result.returncode == 0 and status == 'Accepted':
            return submission
        if status in {'Invalid', 'Rejected'}:
            raise RuntimeError(f'Apple rejected notarization {submission}; inspect its notary log')
        if attempt < 2:
            print(f'Notarization {submission} is not accepted yet; waiting again', flush=True)
            time.sleep(20)
    raise RuntimeError(f'Notarization {submission} did not reach Accepted; no release will be published')


def sign_package(output, config, credentials_dir):
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    packages = list(output.glob(f'yo3mfs-{version}-*-apple-darwin.dmg'))
    if len(packages) != 1:
        raise ValueError('Expected exactly one macOS DMG matching the Cargo version')
    dmg = packages[0]
    bundle = output / 'yo3mfs.app'
    if not (bundle / 'Contents/MacOS/yo3mfs').is_file():
        raise ValueError('Missing packaged application')
    with signing_credentials(config, credentials_dir) as (keychain, authentication):
        signing = ['--sign', config['DEVELOPER_ID_APPLICATION'], '--keychain', str(keychain), '--timestamp']
        run(['codesign', '--force', '--options', 'runtime', *signing, str(bundle)], 'Sign application', config)
        run(['codesign', '--verify', '--deep', '--strict', str(bundle)], 'Verify application signature', config)
        run(['hdiutil', 'create', '-volname', 'YO3MFS', '-srcfolder', str(bundle),
             '-ov', '-format', 'UDZO', str(dmg)], 'Rebuild signed DMG', config)
        run(['codesign', '--force', *signing, str(dmg)], 'Sign DMG', config)
        notarize(dmg, authentication, config)
        # Apple notarizes the outer DMG and the signed application inside it.
        # Stapling the distributed DMG keeps its ticket available offline.
        for attempt in range(3):
            result = run(['xcrun', 'stapler', 'staple', str(dmg)], 'Staple DMG', config, check=False)
            if result.returncode == 0:
                break
            if attempt < 2:
                time.sleep(20)
        else:
            raise RuntimeError('Could not staple the notarization ticket to the DMG')
        run(['xcrun', 'stapler', 'validate', str(dmg)], 'Validate notarization ticket', config)
        run(['spctl', '--assess', '--type', 'open', '--context', 'context:primary-signature',
             '--verbose=4', str(dmg)], 'Gatekeeper DMG assessment', config)
        run(['spctl', '--assess', '--type', 'execute', '--verbose=4', str(bundle)],
            'Gatekeeper application assessment', config)
    # A checksum of the intermediate ad-hoc DMG must never accompany the release.
    with dmg.open('rb') as stream:
        digest = hashlib.file_digest(stream, 'sha256').hexdigest()
    (output / 'SHA256SUMS').write_text(f'{digest}  {dmg.name}\n', encoding='utf-8')
    print(f'Signed, notarized, stapled, and Gatekeeper-verified: {dmg.name}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path, nargs='?')
    parser.add_argument('--cleanup', action='store_true')
    args = parser.parse_args()
    if platform.system() != 'Darwin':
        raise ValueError('Signing and notarization require macOS')
    credentials_dir = Path(os.environ['RUNNER_TEMP']) / 'yo3mfs-signing'
    if args.cleanup:
        cleanup(credentials_dir)
    else:
        if args.directory is None:
            parser.error('directory is required')
        sign_package(args.directory, configuration(os.environ), credentials_dir)


if __name__ == '__main__':
    main()
