# YO3MFS implementation plan

Request dated October 16, 2026: implement the application described by SPEC.md. Work proceeds in sequence, without publication or changes to source 3MF files.

## Version 1.4.0 extension — October 18, 2026

The request adds STL, then STEP, updates all project Markdown, and increases the version. Keep the existing viewer stack and controls. Implement bounded ASCII/binary STL and a sandboxed STEP tessellation adapter; verify geometry/units/placements, rejection/limits/cancellation, localized notes, native opening paths, packaging notices, and hardened macOS runtime. Update Cargo versions and future release links together. No commit, tag, or publication is implied by this implementation request.

The STEP profile and known conversion limits are in [STEP.md](../docs/STEP.md). The sequence below is the completed original implementation history, not a new request to rebuild the app.

## Original sequence

1. Cargo project and testable scene/import contracts.
2. Safe OPC/3MF reading, transforms, units, Production parts, and Bambu metadata.
3. Camera, background settings, and a cancellable loader that publishes only the latest request.
4. Window, GPU viewport, picker/drop/file argument, scene information, and warnings.
5. Real-window checks, macOS packaging, CI, and instructions for other operating systems.
6. Quality checks and documentation synchronized with verified behavior.

## Boundaries

Keep the shared immutable scene separate from the GUI. Use one worker and the single GPU device supplied by eframe. Disable GUI dependencies through a feature flag for core tests. Every import validates limits and references; publish a new scene only after completion.

## Risks

- Unknown Bambu variants: warn or reject according to SPEC and add reproducible fixtures.
- Different GPUs/operating systems: native CI plus separate manual acceptance; macOS evidence does not prove Windows/Linux behavior.
- Large archives: enforce limits during streaming and before buffer allocation.
- Licensing/signing: the owner selects a license; a local build is not presented as a signed public release.

Tasks and actual status are in [todo.md](todo.md). Numeric performance targets remain goals until measured on approved reference hardware.
