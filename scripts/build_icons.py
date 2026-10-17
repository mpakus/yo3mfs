#!/usr/bin/env python3
"""Package approved PNG artwork into icon sizes, ICNS and ICO on macOS.

sips handles resampling and preserves alpha. Python only assembles the ICO
container from those PNG files; it does not redraw or retouch the artwork.
The resulting files are checked in, so normal builds need no image tools.
"""
import shutil
import struct
import subprocess
import tempfile
from pathlib import Path

root = Path(__file__).resolve().parent.parent
branding = root / 'assets' / 'branding'
source = branding / 'yo3mfs-icon.png'
icons = branding / 'icons'
icons.mkdir(exist_ok=True)
for tool in ('sips', 'iconutil'):
    if not shutil.which(tool):
        raise SystemExit(f'{tool} is required to regenerate icons (macOS only)')

# The About logo is shown at 300 logical pixels: 1200 px covers 4x DPI and
# remains below egui's minimum supported maximum texture dimension (2048).
subprocess.run(['sips', '-z', '400', '1200',
                str(branding / 'sources' / 'yo3mfs-logo-dark.png'),
                '--out', str(branding / 'yo3mfs-logo-dark.png')], check=True,
               stdout=subprocess.DEVNULL)

sizes = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
for size in sizes:
    subprocess.run(['sips', '-z', str(size), str(size), str(source),
                    '--out', str(icons / f'{size}.png')], check=True,
                   stdout=subprocess.DEVNULL)

with tempfile.TemporaryDirectory() as temp:
    iconset = Path(temp) / 'yo3mfs.iconset'
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            suffix = '@2x' if scale == 2 else ''
            shutil.copyfile(icons / f'{size * scale}.png',
                            iconset / f'icon_{size}x{size}{suffix}.png')
    subprocess.run(['iconutil', '-c', 'icns', str(iconset),
                    '-o', str(branding / 'yo3mfs.icns')], check=True)

# Windows Vista and later support PNG entries in ICO; 256 is encoded as 0.
ico_sizes = [size for size in sizes if size <= 256]
entries = bytearray(struct.pack('<HHH', 0, 1, len(ico_sizes)))
payload = bytearray()
offset = 6 + 16 * len(ico_sizes)
for size in ico_sizes:
    png = (icons / f'{size}.png').read_bytes()
    entries.extend(struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0,
                               1, 32, len(png), offset))
    payload.extend(png)
    offset += len(png)
(branding / 'yo3mfs.ico').write_bytes(entries + payload)
print(f'Created PNG sizes, ICNS and ICO in {branding}')
