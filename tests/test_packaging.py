"""Package layout/checksum regressions using tiny synthetic executables, not OS builds."""
import contextlib
import io
import os
from pathlib import Path
import runpy
import shutil
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile


ROOT = Path(__file__).resolve().parent.parent


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ['scripts/package.py', 'scripts/verify_package.py', 'docs/PACKAGE.md',
                     'assets/fonts/OFL.txt', 'assets/branding/yo3mfs.ico',
                     'assets/branding/icons/512.png',
                     'assets/linux/com.yo3mfs.viewer.desktop']:
            destination = self.root / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        (self.root / 'Cargo.toml').write_text('[package]\nversion = "1.2.3"\n', encoding='utf-8')
        self.out = self.root / 'dist'

    def run_script(self, script, *args):
        path = self.root / 'scripts' / script
        with patch.object(sys, 'argv', [str(path), *map(str, args)]), contextlib.redirect_stdout(io.StringIO()):
            runpy.run_path(str(path), run_name='__main__')

    def package(self, system, target):
        executable = 'yo3mfs.exe' if system == 'Windows' else 'yo3mfs'
        binary = self.root / 'target' / target / 'release' / executable
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b'Synthetic package-layout fixture; not an executable.\n')
        binary.chmod(0o755)
        with patch('platform.system', return_value=system):
            self.run_script('package.py', '--target', target, '--output-dir', self.out)

    def test_windows_archive_contains_binary_icon_instructions_and_license(self):
        self.package('Windows', 'x86_64-pc-windows-msvc')
        archive_path = self.out / 'yo3mfs-1.2.3-x86_64-pc-windows-msvc.zip'
        with zipfile.ZipFile(archive_path) as archive:
            self.assertEqual(set(archive.namelist()), {
                'yo3mfs-1.2.3/yo3mfs.exe', 'yo3mfs-1.2.3/yo3mfs.ico',
                'yo3mfs-1.2.3/README.md', 'yo3mfs-1.2.3/NotoSansSymbols2-OFL.txt'})
            self.assertEqual(archive.read('yo3mfs-1.2.3/README.md'), (ROOT / 'docs/PACKAGE.md').read_bytes())
        self.run_script('verify_package.py', self.out)

    @unittest.skipIf(os.name == 'nt', 'POSIX executable bits are checked on macOS/Linux')
    def test_linux_archive_preserves_executable_and_launcher_resources(self):
        self.package('Linux', 'x86_64-unknown-linux-gnu')
        archive_path = self.out / 'yo3mfs-1.2.3-x86_64-unknown-linux-gnu.tar.gz'
        with tarfile.open(archive_path) as archive:
            self.assertTrue(archive.getmember('yo3mfs-1.2.3/yo3mfs').mode & 0o111)
        self.run_script('verify_package.py', self.out)

    def test_modified_archive_fails_checksum_verification(self):
        self.package('Windows', 'x86_64-pc-windows-msvc')
        archive_path = self.out / 'yo3mfs-1.2.3-x86_64-pc-windows-msvc.zip'
        with archive_path.open('ab') as archive:
            archive.write(b'changed after packaging')
        with self.assertRaisesRegex(SystemExit, 'checksum mismatch'):
            self.run_script('verify_package.py', self.out)


if __name__ == '__main__':
    unittest.main()
