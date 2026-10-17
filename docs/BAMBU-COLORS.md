# Bambu Studio colors in YO3MFS 1.3

The viewer displays the saved project palette, object/part filament assignments, and `paint_color`, including regions inside an original face. The source 3MF remains unchanged. This is a preview of saved painting, without a brush, new color assignments, or export.

## Format references

Bambu Studio was studied at commit `79df7ae309be398ed30c9f397c13c9911c2edc57`:

- [bbs_3mf.cpp](https://github.com/bambulab/BambuStudio/blob/79df7ae309be398ed30c9f397c13c9911c2edc57/src/libslic3r/Format/bbs_3mf.cpp) — filenames, attributes, object and part metadata.
- [Model.cpp](https://github.com/bambulab/BambuStudio/blob/79df7ae309be398ed30c9f397c13c9911c2edc57/src/libslic3r/Model.cpp) — the XML representation of serialized painting.
- [TriangleSelector.cpp](https://github.com/bambulab/BambuStudio/blob/79df7ae309be398ed30c9f397c13c9911c2edc57/src/libslic3r/TriangleSelector.cpp) — subdivision trees and color states.

The project uses an independently written Rust decoder for the format below; Bambu Studio implementation code was not copied. Research source files and external models remain in ignored `target/` and are not packaged.

## Palette and inheritance

`Metadata/project_settings.config` is JSON. Its `filament_colour` array contains `#RRGGBB` strings. For `#RRGGBBAA`, retain RGB and warn when alpha differs from FF. An invalid entry retains its array position so later filament numbers do not shift.

`Metadata/model_settings.config` is XML. `metadata key="extruder" value="N"` on an `object` assigns its filament; on a `part` it overrides the part's filament. Numbering starts at 1; 0 means inherit. Without an assignment, use the first project filament; without a palette, use neutral material. Resolve a part ID in the context of its parent object. Do not mix local IDs in external Production parts with root-model IDs.

Precedence: nonzero painted-region color → part filament → object filament → first project filament → neutral material. Missing colors produce a warning and fall back to the base material. Print settings and physical AMS spool positions are not simulated. Multicolor filaments use the base color with a warning; silk, gradient, and texture effects are not reproduced.

## The `paint_color` tree

The attribute appears on `triangle` without a namespace. Its uppercase hexadecimal string is read from right to left. Each node starts with one nibble:

- The low two bits are 0 for a leaf, or 1, 2, 3 split sides, yielding 2, 3, 4 child triangles respectively.
- For a leaf, the upper two bits specify state 0, 1, or 2. State 3 is an extension marker: add following nibbles to 3; F continues the number and the first nibble below F terminates it.
- For a subdivision, the upper two bits identify the special vertex, 0–2. A three-side subdivision requires index 0.
- Visit children from last to first, depth-first.

Examples: `0` inherits the part color; `4` means filament 1; `8` means 2; `0C` means 3; `1C` means 4; `0FC` means 18. `481` splits a triangle into regions using filaments 1 and 2.

After cyclically rotating vertices by the special index, call them A, B, C and the edge midpoints AB, BC, CA. Children in stored order are:

| Split sides | Child triangles |
| --- | --- |
| 1 | (A,B,BC), (BC,C,A) |
| 2 | (A,AB,CA), (AB,B,CA), (B,C,CA) |
| 3 | (A,AB,CA), (AB,B,BC), (BC,C,CA), (AB,BC,CA) |

The decoder uses a bounded explicit stack. Truncated, surplus, or invalid symbols cause a warning and base-color rendering of the original face. Exceeding depth, triangle, or memory limits stops import and preserves the previous scene.

## Rendering and limits

Keep original vertices/triangles, bounds, and instances. Subdivision creates display geometry only. Unpainted meshes reuse their original index buffers. Shared meshes are not duplicated for differently colored instances. Colors use packed sRGB; 0 inherits from the instance. Lighting converts sRGB to linear RGB, then to the actual render-target format.

Materials → Project colors / Neutral switches material without importing again. The preference is saved; old settings receive Project colors while retaining language and background. The Triangles metric counts original faces; headless `inspect` separately reports `rendered_triangles` after subdivision and including instances.

Project JSON is limited to 8 MiB, palettes to 256 entries, and paint depth to 64. The shared 10-million rendered-triangle limit applies after subdivision. CPU geometry, staging, GPU buffers, and the previous scene count toward the 2 GiB budget. This estimates structures rather than imposing a hard driver RSS limit.

## Verification and compatibility boundaries

Synthetic tests check exact boundary coordinates, face area/winding, extended numbers, inheritance, shared meshes, degenerate-face removal, fallback, and limits. [external.json](../tests/fixtures/external.json) pins URLs, original ZIP and selected 3MF SHA-256 values, exporter versions, and numeric expectations for five public projects. Geometry of the three new files was compared with 1.2.0; palettes/painted-face counts were checked against XML/JSON.

The tested profile includes selected BambuStudio 01.07.03.04, 01.07.03.50, 01.10.02.76, and 02.04.00.70 files, not every project from those versions. No pixel-for-pixel comparison against a running Bambu Studio was performed. Core base materials, the Materials Extension, textures, G-code preview, and actual printed appearance remain unsupported.

Intersecting volumes are shown as original meshes without the slicer's Boolean operations. Coincident planar surfaces from different parts may exhibit color flicker (z-fighting), observed in the public nameplate example. Reading the palette and paint does not resolve such intersections.
