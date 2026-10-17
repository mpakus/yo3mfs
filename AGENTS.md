# Instructions for agents

@/Users/mpak/.codex/RTK.md

## Context

`yo3mfs` is an offline Rust 3MF viewer for macOS, Linux, and Windows, focused on Bambu Studio projects. The main workflow is to open or drop a file, inspect its geometry from every side, zoom, and change the background.

Version 1.3.1 is implemented. Local evidence and outstanding platform checks are recorded in `docs/VALIDATION.md`. Do not present unverified compatibility as tested. Before working, read [SPEC.md](SPEC.md) and [README.md](README.md), then inspect the actual files and Git state.

## Priorities and scope

- The user's current explicit request defines the work. A request for documentation alone does not authorize implementing the application.
- `SPEC.md` is the product and technical contract. Update it in the same change when requirements change.
- Do not add slicing, geometry editing, printer control, cloud services, authentication, or telemetry without a separate request.
- Complete authorized work without asking again about ordinary reversible actions.
- Preserve other work. Do not use destructive Git commands or rewrite history without an explicit request.
- Do not create commits, releases, or publications merely because documentation lists them as future steps.

## Commands and tools

In the owner's environment, read `/Users/mpak/.codex/RTK.md` and prefix shell commands with `rtk`. Use `rtk proxy <command>` when there is no specialized wrapper. This path is local tooling guidance, not an application dependency. If the file or RTK is unavailable elsewhere, report that and use standard tools. GitHub-hosted CI uses standard commands without RTK.

Use `rg` / `rg --files` through RTK first for searches. For documentation changes, check links, consistency, and the diff.

Standard checks:

```bash
rtk cargo fmt --all -- --check
rtk cargo clippy --locked --all-targets --all-features -- -D warnings
rtk cargo test --locked --all-targets --all-features
rtk cargo build --locked --release
```

Run with `rtk cargo run --locked`. Check a file argument with `rtk cargo run --locked -- "/path/to/model.3mf"`.

## Architecture and Rust

- The chosen stack is `egui/eframe` + `wgpu`. Do not introduce a WebView, JavaScript frontend, or game engine without a justified specification change.
- Use one Cargo package with library modules and a binary entry point. Do not split it into many crates prematurely.
- Use Rust edition 2024. The tested toolchain and declared MSRV are 1.97.1; retain `rust-toolchain.toml` and `Cargo.lock`. Earlier MSRVs are untested.
- Separate `import`, `scene`, `camera`, `renderer`, `settings`, and `app`. Import, scene data, and camera mathematics must be testable without a window or GPU.
- Keep `eframe`, `egui-wgpu`, and `wgpu` compatible. Do not create a second graphics device for the viewport.
- Run ZIP/XML import and geometry preparation outside the UI thread. Do not create unbounded worker queues, scene copies, or GPU buffers.
- Tag load results with request IDs: an outdated load must not replace the most recently selected file.
- Return input failures through `Result`, with typed errors and separate warnings. Do not use `unwrap` / `expect` on file data or OS responses.
- Forbid `unsafe` in project code by default using `#![forbid(unsafe_code)]`. Assess dependencies separately.
- Use English identifiers and `rustfmt`. Comments should explain reasons, units, coordinate systems, and transform order.
- **Write and maintain all project documentation in English.** This includes README, SPEC, AGENTS, validation reports, task documents, and asset documentation. The interface defaults to English; Russian remains selectable in Settings.
- Update **both dictionaries**, `src/i18n/en.rs` and `src/i18n/ru.rs`, in the same change. All UI labels, hints, application messages, errors, and warnings must use typed `Key` values. Do not add user-facing strings directly in UI or importer code. Exceptions: branding, axis names, language names, formatting symbols, and unchanged technical OS/library messages.
- Add keys to both `Key` and `Key::ALL`; translations must use matching placeholders. Both dictionaries are exhaustive, so a missing translation must fail compilation. Check placeholder parity and readable Russian wording.
- Language is stored as `en` / `ru` in `settings.json`. Old settings without a language retain the background and receive English. Switching applies immediately, including to existing errors and warnings.
- Arrow keys pan in screen space when the viewport has focus; Shift accelerates, Alt + arrows orbits. Inputs, menus, and dialogs must not pass these keys to the camera.
- Keep the top bar as an egui MenuBar with YO3MFS / File / View / Help. Do not restore large top buttons or call it a global native macOS menu. **About belongs only in YO3MFS; Help contains Controls.**
- The owner approved the logo and icon on October 16, 2026. Use `assets/branding/`, preserving the copper Y and cube geometry. Keep the original concept and prompts; do not redesign approved graphics without a request. `scripts/build_icons.py` regenerates PNG sizes, ICNS, and ICO on macOS; generated assets are committed. Verify window icons, macOS bundle resources, Windows EXE resources, and the Linux desktop entry with app ID `com.yo3mfs.viewer`.
- About contains the exact link `Made in Austin ✩ Texas` → `https://aomega.co/` in both languages.

## Import contract

- Treat 3MF content as untrusted. Enforce limits while reading, not only against ZIP headers.
- Do not extract archives to arbitrary directories or follow external file references. Disable DTDs and external XML entities.
- Discover the root model through OPC relationships; do not assume `3D/3dmodel.model`.
- Resolve object IDs within their model part. Support instances, nested components, and Production Extension links within the documented profile.
- Preserve geometry, units, and transforms. Do not rearrange objects on a print bed or silently repair a model.
- Reject unknown required extensions with an explanation. Simplified rendering that loses optional visual data must show a persistent warning.
- Keep Bambu-specific logic in a separate adapter. Verify each new format variant with a file of known provenance and exporter version.
- Do not copy Bambu Studio code without a separate license assessment. Studying a format does not authorize copying its implementation.
- Palette, inheritance, and `paint_color` are described in `docs/BAMBU-COLORS.md`. Preserve facet → part → object precedence and local IDs in external model parts. Keep original geometry/statistics separate from GPU subdivisions. Instance colors must not duplicate shared meshes.
- Check new painting variants with exact coordinates/area and real files, not screenshots alone. Include subdivisions and instances in memory/triangle budgets. Invalid optional paint falls back with a warning. Retain Project colors / Neutral and migration of old settings.
- Never overwrite the source 3MF or transmit its contents over the network.

## Verification and completion

- Core checks without GUI: `rtk cargo test --locked --no-default-features`. Package with `rtk proxy python3 scripts/package.py --dmg` on macOS, without `--dmg` on other platforms. Python 3.11+ is required.
- External references: `scripts/check_reference_models.py --download`, using only pinned URLs/hashes from `tests/fixtures/external.json`, stored in ignored `target/`. Do not redistribute them without checking rights.
- Check import with minimal synthetic archives and real Bambu files that can be used for testing; record provenance and SHA-256.
- Verify geometry through instance counts, coordinates, and dimensions, not screenshots alone.
- Separately test invalid archives, cycles, limits, cancellation, and competing open requests.
- Test GUI behavior on macOS, Windows, and Linux; distinguish Linux X11 from Wayland. CI builds do not replace manual dialog, drag-and-drop, or GPU checks.
- Loading, errors, and GPU loss must not silently discard the last successfully loaded scene.
- Documentation-only changes require link, consistency, and diff checks; Rust tests are unnecessary for those changes alone.
- Do not remove failing tests, lower quality limits, or claim platform acceptance just to make a report green.
- Report behavior changes, completed verification, and actual limitations. Distinguish implemented code, passing tests, manual checks, and distribution readiness.

## GitHub builds

- `.github/workflows/ci.yml` must retain four native targets: macOS ARM64, macOS x64, Windows x64, and Linux x64. Do not replace Intel Mac with another Apple Silicon build.
- Read the Rust version from `rust-toolchain.toml` and build with `Cargo.lock`. Pin actions to verified commit SHAs, with release-version comments.
- Keep formatting, Clippy, desktop/core tests, release builds, CLI smoke checks, and package verification before artifact upload. Each archive includes a SHA-256 checksum.
- Build jobs have read-only repository permissions. The owner authorized public GitHub Releases on October 16, 2026: a `vMAJOR.MINOR.PATCH` tag matching Cargo.toml/Cargo.lock and release notes publishes all four packages plus combined SHA256SUMS after the entire matrix passes. Only the release job receives `contents: write`. Branch/PR builds never publish.
- Keep release assets versioned and immutable: resume incomplete drafts, but never replace published assets or move a published tag. Update README download URLs and `docs/releases/vVERSION.md` with every version bump. Do not add signing credentials or private-model downloads without authorization.
- The owner authorized reuse of GitRonimo/Cuckoding/KickstandMoto signing configuration for YO3MFS. GitRonimo supplies the applicable macOS Developer ID/API-key pattern; iOS/EAS profiles are not interchangeable. Follow `docs/MACOS-SIGNING.md`: expose secrets only in tagged macOS signing steps, use an isolated temporary keychain, clean up all credential material, and require notarization/Gatekeeper verification before publication. Never print secret values or attempt to recover GitHub secrets through logs/artifacts. Reuse original files through protected secret inputs; missing credentials must fail closed.
- Keep workflow behavior and artifact names synchronized with `docs/GITHUB-ACTIONS.md`. Report local validation separately from actual GitHub runs and native GUI acceptance.

## Decisions that need separate authorization

Continue ordinary implementation within the user's request. A separate decision is needed to change the user contract: write to 3MF, transmit data, substantially replace the stack, or drop a promised platform. License selection and publication outside the authorized version-tag release workflow require the owner's decision. Prepare a concrete proposal and explain why that decision is needed.
