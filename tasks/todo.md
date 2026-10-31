# YO3MFS tasks

- [x] T1. Cargo + scene: separate meshes/instances and bounds; verify core unit tests.
- [x] T2. Import: ZIP/OPC/XML, units/transforms/components; verify synthetic integration fixtures.
- [x] T3. Compatibility: Production parts, Bambu roles/plates, warnings; verify scene composition and coordinates.
- [x] T4. Camera/settings: views/projection/pan/zoom, atomic settings; verify matrices and persistence.
- [x] T5. Loader: one worker, cancellation, latest request; verify races and failure recovery.
- [x] T6. UI/GPU: model window, picker/drop/file argument, background and statistics; verify native build/window.
- [x] T7. Distribution: macOS bundle, Windows/Linux CI and packaging; verify the available local artifact.
- [x] T8. Review/docs: fmt, Clippy, tests, release build; update README/SPEC/AGENTS and validation evidence.

All eight implementation stages are complete. This does not close full platform acceptance AC-01…AC-12; evidence and limitations are in the [validation report](../docs/VALIDATION.md).

Checks on unavailable operating systems and the owner's licensing decision remain separate from implementation and are not replaced by assumptions.

## Version 1.4.0 — STL and STEP

- [x] Bounded ASCII/binary STL import through the existing loader, picker, drop handler, and file argument.
- [x] Original coordinates/winding, millimeter notice, neutral material, and warnings for unsupported binary attributes.
- [x] EN/RU copy, synthetic fixtures with hashes, numeric geometry checks, negative inputs, limits, and cancellation tests.
- [x] Local desktop/core tests, Clippy, formatting, release build, macOS package verification, picker/startup/error-retention GUI checks, and documentation.
- [ ] Real exporter corpus and native STL acceptance on Intel Mac, Windows, and Linux X11/Wayland; physical drag-and-drop.
- [x] Prepare version 1.4.0 in Cargo files, README download targets, and release notes; published v1.3.1 packages remain 3MF-only.
- [x] STEP/STP CAD tessellation, source units/placements, double-precision recentering, neutral preview, and persistent conversion notes.
- [x] Bounded WASM memory, epoch cancellation/deadline, face-coverage checks, replacement-kernel support, and package license notices.
- [x] Complete automated STEP checks, local package verification, and hardened macOS runtime inspection; record results in validation.
- [x] STEP native Apple Silicon window/picker/startup rendering, camera controls, persistent EN/RU notes, picker cancellation, and error-retention checks after the Mac was unlocked.
- [ ] STEP physical drag-and-drop and actual CAD-conversion cancellation in the UI; native Intel/Windows/Linux acceptance.
- [ ] Native four-target CI, broader CAD exporter corpus, and release distribution/license/signing acceptance.
- [ ] Publish the combined 1.5.0 release only through the owner-authorized reviewed version tag (1.4.0 was not published).

## Version 1.3 — Bambu colors

- [x] Project palette, object/part assignments, and inheritance without duplicating shared meshes.
- [x] Independent bounded `paint_color` decoder for whole faces/subdivisions; preserve original geometry/statistics.
- [x] GPU colors, Project colors / Neutral, inspector palette, saved preference, and both dictionaries.
- [x] Numeric/negative tests, five public references with SHA-256, and manual macOS color/settings checks.
- [x] macOS ARM64/Intel release builds, Windows/Linux cross-target checks, and updated documentation.

Remaining limits: coincident surfaces in intersecting parts may flicker; slicer Boolean operations, painting editing, and physical AMS simulation are not implemented. Platform checks are recorded in the report.

## GitHub preparation

- [x] Keep About only in the YO3MFS menu; retain Controls in Help.
- [x] Translate all project Markdown documentation to English and update the ongoing language rule.
- [x] Configure native macOS ARM64/Intel, Windows x64, and Linux x64 checks, builds, packaging, checksums, and artifact upload.
- [x] First remote workflow run: all four native jobs passed for commit `4aaa266` (run `38999026580`).
- [ ] Perform native GUI acceptance on Intel Mac, Windows, and Linux X11/Wayland.

## Version 1.3.1 — GitHub Releases

- [x] Update Cargo versions, README platform download links, and release notes.
- [x] Publish version-tag builds after all native checks; validate the complete asset set and combined checksums.
- [x] Keep build jobs read-only and limit release write permission to the tag publishing job.
- [x] Publish v1.3.1 and verify all four public downloads against SHA256SUMS; all five README asset URLs return HTTP 200.

## macOS signing follow-up

- [x] Inspect GitRonimo, Cuckoding, and KickstandMoto configuration and adapt the applicable Developer ID/notarization workflow.
- [x] Require signing, notarization, stapling, Gatekeeper checks, and credential cleanup for tagged macOS builds; add negative regression tests.
- [ ] Configure the original signing credentials as YO3MFS repository secrets.
- [ ] Publish a new version with signed Apple Silicon/Intel DMGs and verify downloaded Gatekeeper acceptance.

## OBJ, glTF/GLB, and PLY — October 31, 2026

- [x] Add bounded static importers, scene/color integration, EN/RU messages, and all opening paths.
- [x] Add reproducible geometry fixtures, numeric and malformed-input regressions, and dependency notices.
- [ ] Complete independent exporter and native macOS Intel/Windows/Linux X11/Wayland acceptance; see docs/VALIDATION.md.

## Version 1.5.0 — native checks and release preparation

- [x] Native macOS picker/rendering for all three PLY encodings, embedded glTF, and GLB; expected counts/dimensions and persistent notes.
- [x] Native failed-load scene retention for malformed PLY, external-buffer glTF, and truncated GLB; successful recovery through Choose another file.
- [x] Owner-confirmed physical Finder drops of PLY, glTF, and GLB, including retention of the last good model after dropping invalid GLB; see the validation report.
- [x] Synchronize version 1.5.0, download targets, notices, format documentation, and release notes.
- [x] Prepare the reviewed 1.5.0 source, final local tests/build, and verified macOS development DMG for the requested commit/tag push.
- [x] Push implementation commit `7203c35` and annotated `v1.5.0` tag to GitHub.
- [ ] Configure the missing macOS signing secrets, complete the four-target workflow, and verify release downloads.
