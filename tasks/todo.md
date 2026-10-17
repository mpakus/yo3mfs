# Version 1 tasks

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
- [ ] Publish a new patch version with signed Apple Silicon/Intel DMGs and verify downloaded Gatekeeper acceptance.
