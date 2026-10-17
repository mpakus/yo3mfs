#!/usr/bin/env python3
"""Generate our own curved grid; no external asset or model license required."""
import argparse
import hashlib
import math
import zipfile
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--cells', type=int, default=700)
parser.add_argument('--output', type=Path, default=Path('target/benchmark.3mf'))
args = parser.parse_args()
if not 1 <= args.cells <= 1500:
    raise SystemExit('--cells must be 1..1500')
args.output.parent.mkdir(parents=True, exist_ok=True)
n = args.cells
with zipfile.ZipFile(args.output, 'w', compression=zipfile.ZIP_DEFLATED) as z:
    z.writestr('_rels/.rels', '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="root" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" Target="/3D/model.model"/></Relationships>')
    z.writestr('[Content_Types].xml', '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>')
    with z.open('3D/model.model', 'w') as part:
        part.write(b'<model xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" unit="millimeter"><resources><object id="1" type="model"><mesh><vertices>')
        for y in range(n+1):
            for x in range(n+1):
                height = 10 * math.sin(x/n*math.pi) * math.sin(y/n*math.pi)
                part.write(f'<vertex x="{x/n*100:.6f}" y="{y/n*100:.6f}" z="{height:.6f}"/>'.encode())
        part.write(b'</vertices><triangles>')
        for y in range(n):
            for x in range(n):
                a=y*(n+1)+x; b=a+1; c=a+n+1; d=c+1
                part.write(f'<triangle v1="{a}" v2="{b}" v3="{c}"/><triangle v1="{b}" v2="{d}" v3="{c}"/>'.encode())
        part.write(b'</triangles></mesh></object></resources><build><item objectid="1"/></build></model>')
print(f'{args.output}: {2*n*n} triangles; SHA-256 {hashlib.sha256(args.output.read_bytes()).hexdigest()}')
