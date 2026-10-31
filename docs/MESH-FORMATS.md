# OBJ, PLY, and glTF/GLB static previews

These formats join 3MF, STL, and STEP in the unpublished 1.5.0 source. They use the same background worker, cancellation, latest-request selection, scene preparation, file picker, drag-and-drop, startup path, and headless inspector. Source files are never changed or uploaded. No material library, external buffer, image URI, or other referenced file is opened.

## Supported profile

| Format | Geometry and colors | Boundaries |
| --- | --- | --- |
| Wavefront OBJ (`.obj`) | Polygon faces, positive/negative indices, `v`, `v/vt`, `v//vn`, and `v/vt/vn` corners; homogeneous positions; optional six-value `v x y z r g b` colors in 0–1 sRGB | Objects/groups are combined without moving geometry. `vt`/`vn` references are validated; shading uses face geometry. MTL/textures are not loaded and trigger a persistent notice. Curves, surfaces, include commands, and continuation lines are rejected. |
| Stanford PLY 1.0 (`.ply`) | ASCII, little-endian binary, big-endian binary; typed scalar/list properties; polygon faces using `vertex_indices` or `vertex_index`; optional complete `uchar red/green/blue` vertex colors | Vertex elements must precede face elements. Point-only files receive an explicit surface-mesh error. Unknown properties/elements are consumed within limits and produce a notice when data is omitted. Stored normals, face colors, alpha, and texture data are not rendered. |
| glTF 2.0 / GLB 2 (`.gltf`, `.glb`) | Embedded base64 buffers or GLB BIN data, indexed/unindexed triangles, triangle strips/fans, interleaved and sparse accessors, shared mesh instances, nested node transforms, base-color factors multiplied by `COLOR_0` | Uses the default scene, or first scene if none is designated. Multiple scenes trigger a notice. Only self-contained geometry is loaded; external buffers are rejected with an export-to-GLB hint. No textures, transparency, PBR effects, cameras, animation playback, skins, or morph deformation. Skins/morph targets and unsupported primitive modes are rejected; visual simplification remains in Import notes. Unknown required extensions are rejected; optional extensions warn and use core fallback geometry. |

OBJ and PLY have no reliable unit metadata in this profile. Coordinates remain unchanged and are interpreted as millimeters, with a persistent notice. glTF uses meters and a right-handed Y-up frame: `(x, y, z)` becomes `(1000x, -1000z, 1000y)` in YO3MFS. Nested transforms are composed before this basis conversion; shared meshes are not duplicated per instance.

Concave planar polygons are triangulated with original winding. Self-intersections, nonplanar polygons, invalid indices, nonfinite coordinates, and unusable surfaces fail loading. Unreferenced vertices do not expand displayed surface bounds. Degenerate glTF triangles, including strip connectors, are counted against source limits and skipped with a notice. OBJ/PLY degenerate faces are rejected.

Embedded colors use the existing Project colors / Neutral control. Vertex colors interpolate in linear light; Bambu facet colors remain constant within their painted triangles. Shading remains geometric and double-sided. This is an opaque static preview, not a physically based material renderer.

## Limits and validation

- The existing input, accounted-memory, source-triangle, rendered-triangle, instance, and hierarchy-depth limits apply. All instances count against rendered-triangle limits.
- OBJ reads bounded text lines; PLY reads a bounded buffer before decoding. Both enforce 64 KiB text lines and 1,024 vertices per polygon. PLY headers are capped at 1 MiB, with at most 256 elements and 256 properties per element; element/property names are at most 128 bytes.
- glTF/GLB input and total decoded buffers are capped at 64 MiB, JSON at 16 MiB. The memory budget can reject smaller files. JSON allocations are conservatively charged before parsing; library JSON parsing/validation is bounded and cancellation is checked before/after it. Read, decode traversal, geometry, hierarchy, and preparation loops check cancellation.
- glTF references are range-checked before the dependency can narrow integers. Buffer spans, counts, strides, alignments, sparse ordering, indices, node references, and transforms are validated before typed accessor reads. Required extensions, cycles, duplicate selected-scene node visits, and singular transforms fail clearly.
- Model data is retained only in the current load/scene; nothing is cached on disk. Allocation growth is charged conservatively, including old/new storage during reallocation and vertex-color buffers during scene preparation.

## Sources and dependencies

Implementation references: [Wavefront OBJ documentation](https://www.fileformat.info/format/wavefrontobj/egff.htm), [Stanford PLY description](https://graphics.stanford.edu/data/3Dscanrep/), and the [Khronos glTF 2.0 specification](https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html).

OBJ/PLY parsing and polygon triangulation are project code. glTF uses pinned `gltf 1.4.1` (MIT OR Apache-2.0), with only `utils` and `names`: its filesystem/image importer is disabled. `base64 0.22.1` (MIT OR Apache-2.0, locked in Cargo.lock) decodes embedded buffers. Package notices include the new parser dependencies' license texts; dependency licenses do not select the application license.

## Evidence

`scripts/generate_mesh_fixtures.py` reproducibly creates owned tetrahedra in OBJ, all three PLY encodings, embedded glTF, and GLB. The glTF variants contain two shared instances with a nested translation. [The manifest](../tests/fixtures/manifest.json) records provenance, SHA-256, counts, and expected bounds. These are numeric regression fixtures, not certification of Blender, CAD, scanner, or other exporter versions.

See [VALIDATION.md](VALIDATION.md) for executed checks and remaining native/exporter acceptance. Published 1.3.1 packages do not contain these importers.
