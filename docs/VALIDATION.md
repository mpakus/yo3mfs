# YO3MFS validation

## Original version 1.0.0

Date: October 16, 2026. The implementation and local release build are available. Public release and acceptance on every target platform are not complete.

## Environment and automated checks

Local environment: Apple M4 Max, 64 GiB RAM, macOS 27.0.1, ARM64, Rust/Cargo 1.97.1. The GUI shares eframe's wgpu device; there is no WebView or JavaScript runtime.

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed without warnings |
| `cargo test --locked --all-targets --all-features` | 24 tests passed |
| `cargo test --locked --no-default-features` | 24 tests passed without GUI |
| `cargo build --locked --release` | ARM64 macOS: passed |
| `cargo build --locked --release --target x86_64-apple-darwin` | Intel Mac: cross-build and linking passed; not run on Intel |
| `cargo check --locked --target x86_64-pc-windows-msvc --all-targets` | Passed; no Windows linking/runtime check |
| `cargo check --locked --target x86_64-unknown-linux-gnu --all-targets` | Passed; no Linux linking/runtime check |
| `yo3mfs --version` | `yo3mfs 1.0.0` |
| `scripts/package.py --dmg` | `.app` and `.dmg` created |
| `codesign --verify --deep --strict …/yo3mfs.app` | Passed; ad-hoc signature, not Developer ID |
| `hdiutil verify …/yo3mfs-1.0.0-arm64.dmg` | Valid checksum |

Tests cover OPC root discovery, part-local IDs, nested components, units, rotations/reflections/nonuniform scale, GPU normalization, camera fitting in every view/projection, extreme gestures, settings, cancellation, and load-queue replacement. Negative cases cover cycles, invalid indices/references, NaN, DTDs, truncated XML, external URIs, traversal, archive/XML size, depth, instance/triangle counts, string/XML-token memory, forged ZIP entry counts, EOCD/ZIP64 consistency, and valid ZIP64.

A mutation experiment temporarily changed the inch scale from 25.4 to 1.0. The unit test failed; the source was restored byte-for-byte and final tests passed. Regression tests also reproduced a child model without `build` and excessive warning strings before fixes.

A separate importer review led to fixes for long strings/component copies, XML-token limits before accumulation, per-object Bambu instance numbering, excluding unused vertices from bounds, and deduplicating warnings before allocation. Load publication checks cancellation under the same mutex as opening/canceling requests.

## Manual window checks

Verified on macOS:

- Empty-window startup, Cyrillic text, and enabled/disabled controls.
- System picker and `sample.3mf`: two instances, 24 triangles, 85 × 30 × 42.5 mm.
- Mouse orbit, zoom, fit, top view, and orthographic projection.
- Background presets, HEX through accessibility input, and invalid HEX retaining the previous color.
- White background persistence after exit/restart, then restoration of the dark background.
- Invalid ZIP errors preserve previous geometry and scene information.
- Packaged release `.app` startup. A real project opened by the user displayed four instances and 155,276 triangles. The private file was not copied into the repository; its provenance and geometric accuracy were not independently certified.

Physical drag-and-drop, pinch, all keyboard combinations, a Unicode CLI path, 100%/150%/200% scaling, artificial GPU loss, and clean installation still require separate manual checks. Implemented paths do not replace acceptance evidence.

Surface loss relies on eframe recovery. Device loss displays a system message and closes with restart guidance; automatic device recreation is not implemented. That emergency path has not been artificially triggered.

## Compatibility files

| Sample | Provenance | Numeric result |
| --- | --- | --- |
| `sample.3mf` | Project-authored synthetic geometry; [manifest](../tests/fixtures/manifest.json) | 2 instances, 24 triangles; 85 × 30 × 42.5 mm |
| `pa_pattern.3mf` | [Official BambuStudio pinned resource](https://github.com/bambulab/BambuStudio/blob/79df7ae309be398ed30c9f397c13c9911c2edc57/resources/calib/pressure_advance/pa_pattern.3mf); Application = BambuStudio-01.07.03.04 | 1 instance, 12 triangles; 5 × 5 × 0.85 mm; external Production part without build |
| `flowrate-test-pass1.3mf` | [Official BambuStudio pinned resource](https://github.com/bambulab/BambuStudio/blob/79df7ae309be398ed30c9f397c13c9911c2edc57/resources/calib/filament_flow/flowrate-test-pass1.3mf); exporter version absent | 9 instances, 6,920 triangles; 124 × 94 × 1.4 mm |

Reproduce reference checks with `python3 scripts/check_reference_models.py --download`: pinned URLs, SHA-256 before/after import, and numeric expectations from [external.json](../tests/fixtures/external.json). Files remain in ignored `target/`, outside source/distribution packages. Absent license metadata is not permission to redistribute.

The SHA-256 of `sample.3mf` after GUI checks matched its manifest: `db6c7f83a69dbad2fe453fc63d8aadb6a260ce5f723bc1322fd9cfb4670bc855`. Application code contains no model-upload client and does not open input 3MF files for writing; no separate network-traffic capture was performed. The optional reference downloader is an explicit developer tool, not an application feature.

## Large-model measurement

Project-authored curved mesh generated with `scripts/generate_benchmark.py --cells 700`: 980,000 triangles, one instance, 100 × 100 × 10 mm. Measured archive SHA-256: `f10401f98e38fa5f0bb1a266989f6368d7a7154072f82dcaf7e50a8d74b61366`. ZIP timestamps change the hash on regeneration; geometry is deterministic.

Three separate release `examples/inspect` runs on the above Mac took **0.915 / 0.938 / 0.961 s** for import and preparation. The first `/usr/bin/time -l` run reported maximum resident set size **34,127,872 bytes** (32.55 MiB). Estimated retained CPU geometry: 25,166,842 bytes; planned GPU buffers: 17,656,876 bytes.

These are headless measurements without window creation, GPU upload, or first frame. They do not prove the SPEC p95 frame target or five-second ready-frame goal. Thirty-second rotation and application GPU/RSS measurements remain separate acceptance work.

## Platforms and distribution

CI was configured for native macOS ARM64, macOS Intel, Windows x64, and Ubuntu 24.04 x64. macOS runner labels were checked against the [official GitHub matrix](https://github.com/actions/runner-images). Local cross-target `cargo check` passed for Windows MSVC and Linux GNU; that verifies Rust compilation, not linking or GUI runtime. At this stage there were no remote workflow runs or Windows/Linux CI artifacts. Linux dialogs use [rfd's XDG portal backend](https://github.com/PolyMeilex/rfd); X11/Wayland runtime remained untested.

Initial local `.app`/DMG packages are in `dist/macos-arm64-release/`; Intel builds are in `dist/macos-x86_64-apple-darwin-release/`. `dist/` is ignored by Git. macOS 27 warns that the `hdiutil create` CLI syntax is deprecated, but packaging succeeds; the script retains compatibility with earlier macOS runners.

Public distribution still requires project/dependency-license decisions, signing/notarization, and clean-OS tests. Bundle metadata declares macOS 12 as a target minimum; actual startup has only been checked on 27.0.1. Linux tar.gz does not bundle system libraries or establish portability to other distributions.

## SPEC acceptance status

AC-04 has numeric evidence within the supported profile; AC-12 has synchronized documentation. AC-02/05/07/08/09 have partial local evidence with the limits above. AC-01/03/06/10/11 need expanded manual, format, and platform testing. A single macOS build does not mark these complete.

## Update 1.1.0 — 2026-10-16

Changed arrow-key navigation, inspector styling, language selection, and About. Earlier sections describe 1.0.0; those performance figures were not remeasured for 1.1.0.

- `cargo test --locked --all-targets --all-features`: **34 tests**. Added screen-direction/pan-step checks for all seven views, both projections, and two viewport sizes; Shift/Alt; egui key delivery with viewport focus and no movement with TextEdit focus; saved/migrated language; unknown language; switching existing diagnostics; placeholder parity and no re-interpolation of user strings.
- `cargo test --locked --no-default-features`: **32 tests**. Formatting and Clippy with `-D warnings` passed.
- Final Apple Silicon and Intel release builds linked successfully. Windows MSVC and Linux GNU cross-target checks passed again. Native Windows/Linux/Intel startup was not tested.
- A font test reproduced the missing ✩ glyph before adding Noto Sans Symbols 2, and now checks the star, ⌘, Latin, and Cyrillic. Direction buttons use vector drawing and accessible text labels. The font license is packaged.
- Official Bambu references were rechecked: instance/triangle counts and dimensions matched, and input hashes were unchanged. Headless CLI warnings are now English.
- A real macOS window confirmed English startup with old settings, the new inspector, immediate Russian switching, About 1.1.0, and the exact credit text/star. Escape closed About. The external link was not opened; its URL was checked in code.
- Keyboard focus was tested through automated egui events, and pan through numeric tests. A full manual shortcut/Russian minimum-window/DPI pass remains platform acceptance work. Windows containing user models were not closed.

Final ARM64 package: `dist/1.1.0-macos-arm64-final/`, separate from open intermediate builds. Previous packages remain. Intel: `dist/1.1.0-macos-x86_64-apple-darwin-release/`. Signatures are ad-hoc; no Developer ID/notarization or public release.

## Menu and branding concept 1.2.0 — 2026-10-16

This subsection records the intermediate state before branding approval; later subsections supersede it.

- Replaced top buttons with an in-window egui MenuBar: YO3MFS / File / View / Help. Camera commands are disabled without a scene; orientation/projection use submenus. The right inspector remains. This is not the global macOS menu bar.
- Updated EN/RU dictionaries together. Built-in image_gen created `assets/branding/concepts/yo3mfs-identity-v1.png`; the reviewed concept and prompt were saved. At this intermediate point the assets awaited owner approval and were not yet integrated.
- A separate macOS window verified File, disabled View commands without a model, Russian Help, `⌘,` settings while a menu was open, F1 → Controls, Escape, and `⌘W` closing the empty test window. User-model windows remained open; English was restored afterward.
- **34 tests** passed with all features/targets; fmt and Clippy passed. A full Windows/Linux menu walkthrough remained outstanding.
- Shortcut labels use ⌘ on Mac and Ctrl on Windows/Linux; +, −, and comma appear as actual keys rather than enum names.
- Menu preview package: `dist/1.2.0-menu-preview-macos-arm64/`, still using the previous OS icon. The final approved-icon build is described below.

Apple Silicon and Intel 1.2.0 release builds linked successfully; Windows MSVC/Linux GNU checks passed again. Intel menu-preview package: `dist/1.2.0-menu-preview-macos-x86_64/`. The final Mac menu visibly showed `⌘,` and `⌘Q`. Windows/Linux/Intel runtime was not checked.

## Approved branding 1.2.0

The owner approved use of the logo/icon on October 16, 2026. Production PNG, ICNS, ICO, and prompts are in `assets/branding/`; the original concept board is preserved. About uses a light wordmark on graphite; the empty window uses the transparent icon. The menu remains text-based.

Verified after integration:

- All-feature/all-target tests: **35**, including embedded PNG decoding, corner transparency, and texture loading. A test caught an oversized source logo; the embedded version was reduced to 1200 × 400 and the master retained.
- Core tests without GUI: **32**. Formatting and Clippy with `-D warnings` passed.
- Apple Silicon and Intel release builds linked. Final packages: `dist/1.2.0-macos-arm64/` and `dist/1.2.0-macos-x86_64/`.
- Both bundles passed checks of `CFBundleIconFile`, YO3MFS name, version, byte-identical ICNS, and `codesign --verify --deep --strict`. Ad-hoc only, without Developer ID/notarization.
- Apple Silicon DMG passed `hdiutil verify`; deprecated hdiutil syntax warnings did not prevent successful packaging.
- Windows MSVC/Linux GNU cross-target checks passed. The Windows ICO resource was actually compiled with llvm-rc through `RC_PATH`; all seven PNG representations and the icon group were inspected. No Windows EXE linking/runtime check was performed.
- ICO decodes at 16, 24, 32, 48, 64, 128, and 256 px; ICNS up to 1024 px. PNGs preserve source alpha; some outer pixels retain alpha 1/255.
- The packaged Apple Silicon window visibly showed the icon on a light background, About logo, version 1.2.0, and exact Made in Austin ✩ Texas credit. Window title: YO3MFS. Other model windows were not closed.

Linux packages contain a desktop entry and PNG with matching app ID. Linux X11/Wayland icon behavior, Windows Explorer/taskbar, and Intel Mac startup remain unverified. No remote CI or public publication was performed.

## Bambu colors 1.3.0 — 2026-10-16

Added project palettes, object/part filaments, an independent whole/subdivided-face decoder, GPU colors, and Materials controls. Project colors is the default; Neutral switches without reimport and is saved. Old settings retain background/language. Both dictionaries, README, SPEC, and AGENTS were updated. See [BAMBU-COLORS.md](BAMBU-COLORS.md).

### Automated checks

- All-feature/all-target tests: **46**. Core tests: **43**. Added eleven decoder/integration tests and extended settings tests for migration/persistence.
- Tests check exact color-boundary coordinates, area/winding for 1/2/3-side splits, extended filament numbers, inherited state 0, differently colored instances of a shared external mesh, local IDs, malformed-paint/palette fallback, paint alignment after degenerate-face removal, and subdivision/memory/JSON/palette limits.
- Formatting, Clippy with `-D warnings`, and `git diff --check` passed.
- Apple Silicon and Intel release builds linked. Windows MSVC/Linux GNU cross-target checks passed; Windows used `RC_PATH=/opt/homebrew/opt/llvm/bin/llvm-rc`. These were not native linking/runtime checks on those OSes.
- The five-reference checker matched before/after SHA-256, palettes, instance filaments, painted source-face counts, and rendered-triangle counts. Geometry/dimensions of the three new files matched the 1.2.0 binary. A separate XML pass independently counted paint-tree leaves.

| New sample | Exporter | Source triangles | Painted faces | After subdivision |
| --- | --- | ---: | ---: | ---: |
| Terrier | BambuStudio-01.10.02.76 | 144,206 | 130,369 | 169,462 |
| 3DBenchy | BambuStudio-02.04.00.70 | 225,143 | 1 | 225,143 |
| Nameplate | BambuStudio-01.07.03.50 | 900,712 | 24,618 | 900,712 |

Benchy contains eleven degenerate source triangles, also skipped by 1.2.0. Terrier has 3,238 subdivided source faces; its longest `paint_color` string has 266 symbols. Nameplate has 150 instances and a five-color palette. Exact dimensions, hashes, URLs, and selected external ZIP members are in [external.json](../tests/fixtures/external.json). External models are not committed/bundled; redistribution rights are not assumed.

### Real macOS window and packages

- A separate 1.3.0 `.app` opened Terrier through the system picker: black/white paint boundaries were visible; palette #FFFFFF / #161616, 144,206 source faces, and dimensions matched headless inspection.
- Neutral removed all colors while preserving geometry/view. After closing only the test window and relaunching, Neutral remained selected. Project colors was restored afterward.
- Russian Materials labels and both buttons fit at 1280 × 832. English was restored and the original background retained. User-model windows were not closed.
- Nameplate showed colored parts and 150 instances. Coincident planar surfaces exhibited color flicker because intersecting original volumes are not combined/subtracted as in a slicer. The limitation is documented in README/compatibility notes rather than presented as full Bambu visual equivalence.
- Packages: `dist/1.3.0-macos-arm64/` and `dist/1.3.0-macos-x86_64/`. Version 1.3.0, Mach-O architectures, ICNS identity, strict codesign verification, and both DMGs were verified. Ad-hoc signing only; no Developer ID/notarization/public release.

Repeated headless benchmark of the same 980,000-triangle file: **1.014 s**, maximum resident set size **37,928,960 bytes** (36.17 MiB), planned GPU buffers **19,622,484 bytes**. One run on the same Mac; not a first-frame or p95 rotation measurement. Native Windows/Linux X11/Wayland, Intel Mac, the full drag-and-drop/scaling matrix, and comparison against running Bambu Studio remain unverified.

## English documentation and GitHub build preparation — 2026-10-16

The duplicate Help → About entry was removed; About remains under YO3MFS. All project Markdown documentation was translated to English, including historical evidence, task documents, and asset notes. AGENTS now requires English documentation while retaining both English/Russian UI dictionaries.

The workflow uses four native targets: macOS Apple Silicon/Intel, Windows x64, and Linux x64. It explicitly installs the repository's Rust toolchain and Python 3.13, caches Cargo output, runs format/Clippy/desktop/core checks, builds release binaries, checks `--version`, packages, verifies checksums/structure, and uploads artifacts. Action references are pinned to release SHAs. The workflow has read-only repository permissions and does not automatically publish releases. Portable packages now contain standalone English instructions without broken relative repository links.

The remote repository `mpakus/yo3mfs` was inspected: it is public, has no default-branch commit yet, and has no workflow runs. Local validation is not evidence of a successful GitHub-hosted run. First-push instructions and artifact names are in [GITHUB-ACTIONS.md](GITHUB-ACTIONS.md).

Local verification for this update:

- All **12 project Markdown files** contain English text; relative links and `git diff --check` passed. Original branding prompts and font-license text remain intact.
- `actionlint` **1.7.12** passed on the workflow. Its official release binary was checksum-verified. Pinned action SHAs were resolved from official release tags and their metadata checked; all four use Node.js 24, and artifact upload defaults to an archive containing the package/checksum.
- Rust: **46 all-feature/all-target tests**, **43 core tests**, formatting, and Clippy with `-D warnings` passed.
- Python: **3 packaging regression tests** passed. They use synthetic executable bytes to verify Windows ZIP contents, Linux executable permissions/launcher resources, and rejection of a modified archive. They are packaging logic tests, not Windows/Linux executable builds. The POSIX-permission test runs on macOS/Linux; Windows CI still verifies its own actual ZIP.
- Release builds with explicit `aarch64-apple-darwin` and `x86_64-apple-darwin` targets passed. Both new DMGs passed package/checksum verification, strict codesign checks, and `hdiutil verify`. Packages are in `dist/1.3.0-github-macos-arm64/` and `dist/1.3.0-github-macos-x64/`, separate from earlier open builds.
- A new Apple Silicon app window visibly confirmed Help contains only Controls and YO3MFS contains About. Selecting About opened the dialog with version 1.3.0 and the unchanged credit link. User-model windows and existing preferences were not altered.

These results do not claim a hosted workflow run, native Intel startup, or Windows/Linux GUI acceptance. No commit, push, tag, signing credential, or public release was created by this update.

## Release publishing 1.3.1 — 2026-10-16

The owner requested publishing build packages on GitHub Releases and linking them from README. The preceding [1.3.0 workflow run](https://github.com/mpakus/yo3mfs/actions/runs/38999026580) was inspected: macOS Apple Silicon, macOS Intel, Windows x64, and Linux x64 all passed on commit `4aaa266`. This supersedes the earlier empty-repository snapshot without changing the historical results above.

Version 1.3.1 adds version-tag publication after the complete native build matrix, a strict four-package/version/checksum gate, combined SHA256SUMS, release notes, and direct README downloads. Uploads go to a draft before it becomes public. An interrupted draft can be resumed; existing published assets are never overwritten. Build jobs retain read-only permissions, with write access limited to the release job. The new download action is pinned to the verified official v8.0.2 commit.

Local pre-publication checks passed: **46 Rust tests**, **12 Python packaging/release tests**, formatting, Clippy with `-D warnings`, actionlint, tag/Cargo version agreement, and local documentation links. Regression coverage includes mismatched tags/lockfiles, missing notes/platforms, wrong-version or modified packages, draft resumption, immutable published releases, and failed uploads staying unpublished. Publication and downloaded-asset verification are recorded separately after the hosted run finishes. Signing and remaining manual platform acceptance are unchanged.


### Hosted release and public downloads

The [v1.3.1 tag run](https://github.com/mpakus/yo3mfs/actions/runs/39000331367) passed all five jobs: macOS Apple Silicon, macOS Intel, Windows x64, Linux x64, and Publish GitHub Release. The annotated tag points to commit `9391ec05b0e3bf4d2b71dd27e22cc813e618048c`. Each native build executed its CLI version smoke test and package verification before upload.

[YO3MFS v1.3.1](https://github.com/mpakus/yo3mfs/releases/tag/v1.3.1) was published as Latest on October 16, 2026 (America/Chicago). It is public, not a draft or prerelease, with exactly four native packages and `SHA256SUMS`. All four packages were downloaded from the Release and their hashes matched the combined checksum file. The five direct asset URLs in README returned HTTP 200 without authentication. The published README contains the release links and existing screenshot.

The redundant branch run for the same release commit was canceled after the tag run started; the successful tag run is the release evidence. Documentation updates after publication do not move the release tag or change its binaries. Signing, manual Intel/Windows/Linux GUI acceptance, and licensing decisions remain outstanding as documented above.
