# Version 1 implementation plan

Request dated October 16, 2026: implement the application described by SPEC.md. Work proceeds in sequence, without publication or changes to source 3MF files.

## Sequence

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
