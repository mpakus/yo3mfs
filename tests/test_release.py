"""Release gates: complete, versioned, untampered packages before publication."""
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location('release', Path(__file__).resolve().parents[1] / 'scripts/release.py')
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'Cargo.toml').write_text('[package]\nversion = "1.2.3"\n')
        (self.root / 'Cargo.lock').write_text('[[package]]\nname = "yo3mfs"\nversion = "1.2.3"\n')
        self.notes = self.root / 'docs/releases/v1.2.3.md'
        self.notes.parent.mkdir(parents=True)
        self.notes.write_text('Release notes')
        self.artifacts = self.root / 'artifacts'
        self.output = self.root / 'release'
        for artifact, suffix in release.TARGETS.items():
            directory = self.artifacts / artifact
            directory.mkdir(parents=True)
            name = f'yo3mfs-1.2.3-{suffix}'
            data = f'Synthetic fixture: {artifact}'.encode()
            (directory / name).write_bytes(data)
            (directory / 'SHA256SUMS').write_text(f'{hashlib.sha256(data).hexdigest()}  {name}\n')

    def test_matching_tag_and_complete_packages_produce_five_release_assets(self):
        version, notes = release.check_tag(self.root, 'v1.2.3')
        self.assertEqual(notes, self.notes)
        assets = release.prepare(self.artifacts, self.output, version)
        self.assertEqual(len(assets), 5)
        lines = (self.output / 'SHA256SUMS').read_text().splitlines()
        self.assertEqual(len(lines), 4)
        for line in lines:
            digest, name = line.split('  ')
            self.assertEqual(digest, hashlib.sha256((self.output / name).read_bytes()).hexdigest())

    def test_invalid_or_mismatched_tags_are_rejected(self):
        for tag in ['v1.2.4', '1.2.3', 'v1.2.3-rc.1', 'main', 'v1.2.3\n']:
            with self.subTest(tag=tag), self.assertRaisesRegex(ValueError, 'Release tag'):
                release.check_tag(self.root, tag)

    def test_lockfile_and_notes_must_match_the_release(self):
        (self.root / 'Cargo.lock').write_text('[[package]]\nname = "yo3mfs"\nversion = "1.2.2"\n')
        with self.assertRaisesRegex(ValueError, 'Cargo.lock'):
            release.check_tag(self.root, 'v1.2.3')
        (self.root / 'Cargo.lock').write_text('[[package]]\nname = "yo3mfs"\nversion = "1.2.3"\n')
        self.notes.unlink()
        with self.assertRaisesRegex(ValueError, 'Missing release notes'):
            release.check_tag(self.root, 'v1.2.3')

    def test_missing_platform_cannot_prepare_a_partial_release(self):
        (self.artifacts / 'yo3mfs-macos-x64').rename(self.artifacts / 'unexpected')
        with self.assertRaisesRegex(ValueError, 'four native'):
            release.prepare(self.artifacts, self.output, '1.2.3')
        self.assertFalse(self.output.exists())

    def test_modified_package_is_rejected_before_output_is_written(self):
        package = next((self.artifacts / 'yo3mfs-linux-x64').glob('*.tar.gz'))
        package.write_bytes(b'modified')
        with self.assertRaisesRegex(ValueError, 'Checksum mismatch'):
            release.prepare(self.artifacts, self.output, '1.2.3')
        self.assertFalse(self.output.exists())

    def test_wrong_version_packages_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unexpected package'):
            release.prepare(self.artifacts, self.output, '1.2.4')

    @patch.object(release.subprocess, 'run')
    @patch.object(release.subprocess, 'check_output', return_value='[[]]')
    def test_new_release_is_drafted_uploaded_then_published(self, lookup, run):
        release.publish('owner/repo', 'v1.2.3', self.notes, [self.output / 'SHA256SUMS'])
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual([command[2] for command in commands], ['create', 'upload', 'edit'])
        self.assertIn('--draft', commands[0])
        self.assertIn('--verify-tag', commands[0])
        self.assertIn('--draft=false', commands[2])

    @patch.object(release.subprocess, 'run')
    @patch.object(release.subprocess, 'check_output')
    def test_existing_draft_resumes_but_published_release_is_never_changed(self, lookup, run):
        for draft in [True, False]:
            lookup.return_value = json.dumps([[{'tag_name': 'v1.2.3', 'draft': draft}]])
            run.reset_mock()
            if draft:
                release.publish('owner/repo', 'v1.2.3', self.notes, [])
                self.assertEqual([c.args[0][2] for c in run.call_args_list], ['upload', 'edit'])
            else:
                with self.assertRaisesRegex(ValueError, 'already published'):
                    release.publish('owner/repo', 'v1.2.3', self.notes, [])
                run.assert_not_called()

    @patch.object(release.subprocess, 'run')
    @patch.object(release.subprocess, 'check_output', return_value='[[]]')
    def test_upload_failure_keeps_release_as_draft(self, lookup, run):
        run.side_effect = [None, subprocess.CalledProcessError(1, 'gh release upload')]
        with self.assertRaises(subprocess.CalledProcessError):
            release.publish('owner/repo', 'v1.2.3', self.notes, [])
        self.assertEqual([c.args[0][2] for c in run.call_args_list], ['create', 'upload'])


if __name__ == '__main__':
    unittest.main()
