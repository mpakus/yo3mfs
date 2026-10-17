"""Signing workflow boundaries without accessing certificates or Apple services."""
import base64
from contextlib import contextmanager
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location('sign_macos', Path(__file__).resolve().parents[1] / 'scripts/sign_macos.py')
signing = importlib.util.module_from_spec(spec)
spec.loader.exec_module(signing)


def result(code=0, payload=None, stderr=''):
    return subprocess.CompletedProcess([], code, json.dumps(payload or {}), stderr)


class MacSigningTests(unittest.TestCase):
    def setUp(self):
        self.config = {name: f'private-{name.lower()}' for name in signing.SECRET_NAMES}
        self.config['DEVELOPER_ID_APPLICATION'] = 'Developer ID Application: Test Identity (TESTTEAM)'
        for name in ['MACOS_CERTIFICATE_BASE64', 'APPLE_API_KEY_BASE64']:
            self.config[name] = base64.b64encode(b'synthetic fixture, not a key').decode()
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_configuration_requires_all_secrets_and_developer_id(self):
        self.assertEqual(signing.configuration(self.config), self.config)
        for name in signing.SECRET_NAMES:
            with self.subTest(name=name):
                config = self.config.copy()
                del config[name]
                with self.assertRaisesRegex(ValueError, name):
                    signing.configuration(config)
        config = dict(self.config, DEVELOPER_ID_APPLICATION='Apple Development: wrong certificate')
        with self.assertRaisesRegex(ValueError, 'Developer ID Application'):
            signing.configuration(config)

    def test_invalid_certificate_encoding_is_rejected_without_echoing_value(self):
        config = dict(self.config, MACOS_CERTIFICATE_BASE64='invalid-secret-value!')
        with self.assertRaises(ValueError) as error:
            signing.configuration(config)
        self.assertNotIn(config['MACOS_CERTIFICATE_BASE64'], str(error.exception))

    @patch.object(signing.subprocess, 'run')
    def test_command_errors_redact_secrets_and_do_not_include_command(self, run):
        password = self.config['MACOS_CERTIFICATE_PASSWORD']
        run.return_value = result(1, stderr=f'failed with {password}')
        with self.assertRaises(RuntimeError) as error:
            signing.run(['security', 'import', '-P', password], 'Import certificate', self.config)
        self.assertIn('[redacted]', str(error.exception))
        self.assertNotIn(password, str(error.exception))
        self.assertNotIn('security', str(error.exception))

    @patch.object(signing, 'run', side_effect=RuntimeError('Import failed'))
    def test_partial_setup_removes_sensitive_files(self, run):
        directory = self.root / 'credentials'
        with self.assertRaisesRegex(RuntimeError, 'Import failed'):
            with signing.signing_credentials(self.config, directory):
                self.fail('Failed setup must not yield signing credentials')
        self.assertFalse(directory.exists())

    @patch.object(signing.subprocess, 'run', return_value=result(1))
    def test_keychain_cleanup_failure_still_removes_private_key_files(self, run):
        directory = self.root / 'credentials'
        directory.mkdir()
        (directory / 'release.keychain-db').write_bytes(b'synthetic keychain')
        (directory / 'api-key.p8').write_bytes(b'synthetic key')
        with self.assertRaisesRegex(RuntimeError, 'temporary credential files were removed'):
            signing.cleanup(directory)
        self.assertFalse(directory.exists())

    @patch.object(signing.time, 'sleep')
    @patch.object(signing, 'run')
    def test_timeout_polls_same_submission_without_uploading_again(self, run, sleep):
        run.side_effect = [result(payload={'id': 'submission-1'}),
                           result(1, {'status': 'In Progress'}), result(payload={'status': 'Accepted'})]
        self.assertEqual(signing.notarize(Path('model.dmg'), [], self.config), 'submission-1')
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual([c[2] for c in commands], ['submit', 'wait', 'wait'])
        self.assertEqual([c[3] for c in commands[1:]], ['submission-1', 'submission-1'])

    @patch.object(signing, 'run')
    def test_rejected_notarization_stops_immediately(self, run):
        run.side_effect = [result(payload={'id': 'submission-1'}), result(payload={'status': 'Invalid'})]
        with self.assertRaisesRegex(RuntimeError, 'Apple rejected'):
            signing.notarize(Path('model.dmg'), [], self.config)
        self.assertEqual(run.call_count, 2)

    @patch.object(signing.time, 'sleep')
    @patch.object(signing, 'run')
    def test_unknown_notary_status_never_counts_as_success(self, run, sleep):
        run.side_effect = [result(payload={'id': 'submission-1'})] + [result()] * 3
        with self.assertRaisesRegex(RuntimeError, 'did not reach Accepted'):
            signing.notarize(Path('model.dmg'), [], self.config)

    def package_fixture(self):
        (self.root / 'Cargo.toml').write_text('[package]\nversion = "1.2.3"\n')
        output = self.root / 'package'
        binary = output / 'yo3mfs.app/Contents/MacOS/yo3mfs'
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'synthetic executable')
        (output / 'yo3mfs-1.2.3-aarch64-apple-darwin.dmg').write_bytes(b'synthetic disk image')
        (output / 'SHA256SUMS').write_text('intermediate checksum')
        return output

    @patch.object(signing, 'notarize', side_effect=RuntimeError('Not accepted'))
    @patch.object(signing, 'run', return_value=result())
    def test_failed_notarization_never_rewrites_release_checksum(self, run, notarize):
        output = self.package_fixture()

        @contextmanager
        def credentials(*args):
            yield self.root / 'keychain', []

        with patch.object(signing, 'ROOT', self.root), patch.object(signing, 'signing_credentials', credentials):
            with self.assertRaisesRegex(RuntimeError, 'Not accepted'):
                signing.sign_package(output, self.config, self.root / 'credentials')
        self.assertEqual((output / 'SHA256SUMS').read_text(), 'intermediate checksum')
        commands = [call.args[0] for call in run.call_args_list]
        self.assertIn('runtime', commands[0])
        self.assertFalse(any('stapler' in c or 'spctl' in c for c in commands))

    @patch.object(signing, 'notarize', return_value='submission-1')
    @patch.object(signing, 'run', return_value=result())
    def test_checksum_covers_final_stapled_dmg_only_after_gatekeeper(self, run, notarize):
        output = self.package_fixture()
        dmg = next(output.glob('*.dmg'))

        @contextmanager
        def credentials(*args):
            yield self.root / 'keychain', []

        def execute(command, *args, **kwargs):
            if command[:3] == ['xcrun', 'stapler', 'staple']:
                dmg.write_bytes(b'final signed and stapled fixture')
            if command[0] == 'spctl':
                self.assertEqual((output / 'SHA256SUMS').read_text(), 'intermediate checksum')
            return result()

        run.side_effect = execute
        with patch.object(signing, 'ROOT', self.root), patch.object(signing, 'signing_credentials', credentials):
            signing.sign_package(output, self.config, self.root / 'credentials')
        expected = hashlib.sha256(dmg.read_bytes()).hexdigest()
        self.assertEqual((output / 'SHA256SUMS').read_text(), f'{expected}  {dmg.name}\n')
        assessments = [c.args[0] for c in run.call_args_list if c.args[0][0] == 'spctl']
        self.assertEqual(len(assessments), 2)


if __name__ == '__main__':
    unittest.main()
