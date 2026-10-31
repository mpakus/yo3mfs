#!/usr/bin/env python3
"""Reproduce owned OBJ/PLY/glTF fixtures and their provenance (Python 3.11+)."""
import base64
import hashlib
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / 'tests/fixtures'
POINTS = [(-2, -3, -4), (8, -3, -4), (-2, 17, -4), (-2, -3, 26)]
COLORS = [(255, 80, 40), (40, 220, 80), (40, 100, 255), (250, 220, 40)]
FACES = [(0, 2, 1), (0, 1, 3), (0, 3, 2), (1, 2, 3)]


def main():
    manifest = json.loads((FIXTURES / 'manifest.json').read_text())

    def save(name, data, encoding, instances=1):
        (FIXTURES / name).write_bytes(data)
        manifest[name] = {
            'source': 'Synthetic tetrahedron authored by scripts/generate_mesh_fixtures.py; no third-party model',
            'exporter': 'YO3MFS mesh fixture generator v1, 2026-10-31; not exporter interoperability evidence',
            'encoding': encoding,
            'sha256': hashlib.sha256(data).hexdigest(),
            'instances': instances,
            'triangles': 4 * instances,
            'bounds_mm': {'min': [-2, -3, -4], 'max': [8 + 40 * (instances - 1), 17, 26]},
            'tolerance_mm': 1e-5,
        }

    obj = '# YO3MFS synthetic colored tetrahedron\no tetrahedron\n'
    for p, rgb in zip(POINTS, COLORS):
        obj += 'v ' + ' '.join(map(str, p)) + ' ' + ' '.join(str(c / 255) for c in rgb) + '\n'
    for face in FACES:
        obj += 'f ' + ' '.join(str(i + 1) for i in face) + '\n'
    save('sample.obj', obj.encode(), 'OBJ polygons with RGB vertex extension')

    for encoding, name, endian in [('ascii', 'sample-ascii.ply', None),
                                    ('binary_little_endian', 'sample-little.ply', '<'),
                                    ('binary_big_endian', 'sample-big.ply', '>')]:
        data = bytearray(f'ply\nformat {encoding} 1.0\ncomment YO3MFS synthetic colored tetrahedron\nelement vertex 4\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face 4\nproperty list uchar int vertex_indices\nend_header\n'.encode())
        for point, rgb in zip(POINTS, COLORS):
            data += (struct.pack(endian + 'fffBBB', *point, *rgb) if endian else
                     (' '.join(map(str, (*point, *rgb))) + '\n').encode())
        for face in FACES:
            data += (struct.pack(endian + 'Biii', 3, *face) if endian else
                     ('3 ' + ' '.join(map(str, face)) + '\n').encode())
        save(name, data, encoding)

    # glTF is right-handed Y-up in meters; inverse of the viewer's basis conversion.
    binary = b''.join(struct.pack('<fff', x / 1000, z / 1000, -y / 1000) for x, y, z in POINTS)
    binary += b''.join(struct.pack('<HHH', *face) for face in FACES)
    doc = {
        'asset': {'version': '2.0', 'generator': 'YO3MFS mesh fixture generator v1'},
        'buffers': [{'byteLength': len(binary)}],
        'bufferViews': [{'buffer': 0, 'byteOffset': 0, 'byteLength': 48}, {'buffer': 0, 'byteOffset': 48, 'byteLength': 24}],
        'accessors': [
            {'bufferView': 0, 'componentType': 5126, 'count': 4, 'type': 'VEC3', 'min': [-0.002, -0.004, -0.017], 'max': [0.008, 0.026, 0.003]},
            {'bufferView': 1, 'componentType': 5123, 'count': 12, 'type': 'SCALAR'},
        ],
        'materials': [{'pbrMetallicRoughness': {'baseColorFactor': [0.8, 0.2, 0.05, 1]}}],
        'meshes': [{'primitives': [{'attributes': {'POSITION': 0}, 'indices': 1, 'material': 0}]}],
        'nodes': [{'mesh': 0, 'name': 'Original'}, {'translation': [0.04, 0, 0], 'children': [2]}, {'mesh': 0, 'name': 'Translated instance'}],
        'scenes': [{'nodes': [0, 1]}], 'scene': 0,
    }
    packed = json.dumps(doc, separators=(',', ':')).encode()
    packed += b' ' * (-len(packed) % 4)
    glb = struct.pack('<4sII', b'glTF', 2, 12 + 8 + len(packed) + 8 + len(binary))
    glb += struct.pack('<I4s', len(packed), b'JSON') + packed + struct.pack('<I4s', len(binary), b'BIN\0') + binary
    save('sample.glb', glb, 'GLB 2.0 with shared geometry and nested placement', 2)
    doc['buffers'][0]['uri'] = 'data:application/octet-stream;base64,' + base64.b64encode(binary).decode()
    save('sample.gltf', (json.dumps(doc, indent=2) + '\n').encode(), 'glTF 2.0 with embedded base64 buffer', 2)
    (FIXTURES / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')


if __name__ == '__main__':
    main()
