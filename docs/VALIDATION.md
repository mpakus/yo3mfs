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

### macOS Gatekeeper report

After publication, the owner reported macOS blocking YO3MFS because Apple could not verify it. The downloaded Apple Silicon 1.3.1 DMG was mounted read-only for inspection: `codesign --verify --deep --strict` passed, while signature details reported `Signature=adhoc` and no TeamIdentifier. `spctl --assess --type execute --verbose=4` rejected the app. A valid ad-hoc signature and passing build checks do not establish Gatekeeper approval.

README, portable-package instructions, and release notes now describe Apple's app-specific Privacy & Security → Open Anyway flow. This is an installation workaround; the published binaries remain unchanged and unnotarized. The permanent distribution fix requires Developer ID signing and successful Apple notarization, followed by ticket stapling and Gatekeeper verification. The user's local security settings were not changed, and successful launch through the exception has not been independently observed.

## Reusing macOS signing configuration — 2026-10-16

Inspected the requested reference projects. GitRonimo's release workflow supplies the applicable Developer ID/Application and App Store Connect API-key schema; its [v2.0.6 run](https://github.com/mpakus/gitronimo/actions/runs/35029907645) passed. Cuckoding's current Tauri/macOS configuration is for ad-hoc local builds, and KickstandMoto's EAS configuration targets iOS rather than macOS Developer ID distribution. These source repositories were not modified.

YO3MFS now adapts GitRonimo's seven secret names, with tag-only signing, an isolated temporary keychain, hardened runtime/timestamps, a single notarization submission with bounded waits, DMG stapling, Gatekeeper checks, cleanup, and final checksums. Tagged macOS verification independently requires Developer ID/hardened runtime and a valid stapled ticket. Release publication still depends on all four native jobs passing; no unsigned fallback is allowed for new tags.

Local verification: **23 Python packaging/release/signing tests**, actionlint, documentation links, and `git diff --check` passed. Signing tests use synthetic credentials and mocked OS/Apple commands; they cover missing/invalid secrets, redaction, cleanup failures, timeouts without duplicate submissions, rejected/unknown notary status, and checksumming only after Gatekeeper acceptance. They are not evidence of actual Apple notarization.

Credential availability remains the blocker for an end-to-end signed release. GitRonimo has the seven repository secret names; YO3MFS has none configured. GitHub does not expose saved values for copying. No original signing/API-key files were found in the inspected project roots or standard local signing-file locations. A local Developer ID identity exists, but exportable certificate material and usable notarization credentials have not been supplied to this workflow. No credentials were exported or changed; published v1.3.1 binaries and its tag remain unchanged.

## Downey.jr signing comparison — 2026-10-17

The requested Downey.jr checkout maps to `mpakus/1537paperstreet`. Its local release workflow is byte-for-byte identical to the workflow at the successful [v0.8.26 release commit](https://github.com/mpakus/1537paperstreet/actions/runs/37828380069). GitHub reports successful credential preparation, Tauri signing/notarization, DMG notarization/stapling, and distribution verification. All seven repository secret names match the owner's screenshot and YO3MFS's existing signing inputs. Tauri aliases six inputs and manages its own keychain; the stored `KEYCHAIN_PASSWORD` is unused there, whereas YO3MFS uses it explicitly. Details are in [MACOS-SIGNING.md](MACOS-SIGNING.md).

YO3MFS's [signing-configuration branch run](https://github.com/mpakus/yo3mfs/actions/runs/39003262549) passed all four native build jobs. As intended for a branch run, signing/notarization and release publication were skipped. This establishes hosted build/test/package success, not Apple acceptance.

The live YO3MFS repository secret list remains empty. No original `.p12`, `.p8`, or environment configuration files were found in the inspected Downey checkout outside build/dependency directories. No source workflow changes were needed for compatibility with these secret names. This comparison changed only documentation; no credentials, reference-project files, release tags, or published assets were changed.

## Unreleased STL support — 2026-10-18

The source checkout now imports ASCII and little-endian binary STL through the existing bounded background loader. The picker, empty-window/drop copy, About text, CLI help, and both dictionaries include STL. Coordinates/winding are preserved; unitless coordinates use millimeters with a persistent notice. Multiple ASCII solids remain in one mesh. Nonstandard binary colors/attributes use neutral material with a warning. No dependency, package version, published tag, or release asset changed.

Local checks on Apple Silicon/macOS:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed without warnings |
| `cargo test --locked --all-targets --all-features` | 58 passed, including 12 STL tests |
| `cargo test --locked --no-default-features` | 55 passed without GUI/GPU |
| `cargo build --locked --release --examples --bins` | Passed |
| `python3 -m unittest discover -s tests -p 'test_*.py'` | 23 passed; signing/Apple interactions remain mocked |
| Release `--help` | Both file formats shown |
| Release headless inspection | Both STL fixtures: 1 instance, 4 triangles, 10 × 20 × 30 mm; existing 3MF fixture: 2 instances, 24 triangles, 85 × 30 × 42.5 mm |
| `scripts/package.py --dmg --output-dir target/stl-validation/package` and `scripts/verify_package.py target/stl-validation/package` | Development app/DMG, SHA-256, ICNS resource, ad-hoc signature, and DMG integrity passed |
| Documentation relative links, fixture SHA-256, and `git diff --check` | Passed |

The project-authored ASCII/binary tetrahedra are in `tests/fixtures/`, with provenance and SHA-256 in [manifest.json](../tests/fixtures/manifest.json). Bounds are `[-2, -3, -4]` to `[8, 17, 26]`; surface area is 900 mm² and signed volume is 1,000 mm³. The binary fixture intentionally starts with `solid`. Tests also cover ZIP-like binary headers, multiple solids, CRLF/scientific notation, Unicode/spaced paths, misleading extensions, unchanged source bytes, truncation at every byte of a binary triangle, forged counts/trailing bytes, incomplete ASCII geometry, nonfinite/range failures, degenerates/empty geometry, input/line/triangle/instance/memory/rendering limits, streaming cancellation, input growth after size inspection, and persistent EN/RU warnings. Existing latest-request/cancellation and 3MF/Bambu tests still pass.

The initial STL tests failed against the 3MF-only loader before implementation. A separate review caught ZIP-signature bytes inside valid binary STL headers and an accidental change to the Core 3MF diagnostic. Both were corrected; a failing header-dispatch regression was verified before the fix. An inverted binary-length condition also caused the focused geometry test to fail; the source was restored and the final checks above passed.

Native macOS UI checks used the locally built app: binary and ASCII fixtures open through the system picker; the expected filename/counts/dimensions appear and geometry is visible; Top/Orthographic views work; Import notes retain the millimeter assumption. Loading an incomplete STL displays a typed error while keeping the previous ASCII model visible with its original filename/statistics. Launching the packaged executable with the binary STL path opens that scene at startup. The existing background preference was retained.

A generated 1,000,000-triangle binary STL (50,000,084 bytes; SHA-256 `a2ffede7c078ab14a5bcdbc6a4a35ee5860290c44634121bd76fee48c003190f`) was inspected once with the release example: import 0.037 s, import plus geometry preparation 0.046 s, maximum RSS 135,069,696 bytes. It repeats one synthetic triangle and is only a parser/allocation smoke measurement, not a real-model, first-frame, or FPS benchmark.

Limitations: no real CAD/Bambu STL exporter corpus was tested, and no new GitHub or native Intel Mac/Windows/Linux run was performed. Physical drag-and-drop, gestures/DPI, and GPU-loss injection remain unverified for STL. macOS packaging emitted the existing `hdiutil create` deprecation warning but completed and verified successfully. The local development artifact still carries package version 1.3.1 and is ad-hoc signed; it is not a new public release or notarization result. Published v1.3.1 downloads remain 3MF-only.

## Version 1.4.0 — STL and STEP preview, 2026-10-18

This source change combines the STL implementation above with STEP/STP preview and increases Cargo.toml/Cargo.lock to **1.4.0**. All existing project Markdown files were reviewed and updated; historical release/signing records remain distinct. README download targets and [release notes](releases/v1.4.0.md) describe a prepared, unpublished version. No commit, tag, release, credential change, or upload was performed.

STEP uses pinned occt-wasm 5.0.1 and Wasmtime 48.0.6 through a safe Rust host. The format profile, conversion/healing limitations, sandbox limits, and source/license/replacement instructions are in [STEP.md](STEP.md). Import notes always disclose neutral tessellation, flattened assembly structure, omitted metadata/external files, and the upstream facade's inability to verify full root transfer. Imported-face coverage is checked separately before accepting a mesh.

Local checks on the same Apple Silicon/macOS environment:

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed without warnings |
| `cargo test --locked --all-targets --all-features` | 67 passed, including 6 STEP integration tests and 3 sandbox tests |
| `cargo test --locked --no-default-features` | 64 passed without GUI/GPU |
| `cargo build --locked --release --bins --example inspect` | Passed; `yo3mfs --version` reports 1.4.0 |
| Python packaging/release/signing regressions | 23 passed; Apple services and certificate operations are mocked |
| `target/actionlint/actionlint .github/workflows/ci.yml` | Passed with actionlint 1.7.12 |
| STEP release inspection | Box: 12 triangles, 10 × 20 × 30 mm; cylinder: 628 triangles, 10 × 9.998023 × 20 mm |
| Hardened macOS runtime | Ad-hoc signed inspection executable with the exact release entitlement successfully imported STEP; 12 triangles and exact box dimensions |
| Development package and verifier | 1.4.0 ARM64 app/DMG, checksum, ICNS, font/STEP notices/licenses, ad-hoc signature, and DMG integrity passed |
| Native macOS ARM64 STEP viewer | Packaged app: STP picker load, STEP startup argument, rendered cylinder/assembly, camera controls, persistent EN/RU notes, malformed-input scene retention, and picker cancellation passed |
| Markdown local links, version consistency, fixture SHA-256, workflow syntax, and diff whitespace | Passed |

The four STEP fixtures are authored by `examples/generate_step_fixtures.rs` and exported by the pinned Open CASCADE 8.0 kernel as AP214 `AUTOMOTIVE_DESIGN`. [manifest.json](../tests/fixtures/manifest.json) records exact SHA-256 and expected bounds. The box spans `[-2, -3, -4]` to `[8, 17, 26]` with signed volume 6,000 mm³. A repeated-placement compound spans 50 × 20 × 30 mm with 24 triangles. A second box is offset by one billion millimeters; double-precision recentering preserves its 10 × 20 × 30 mm size. Cylinder bounds are checked within tessellation tolerance. These are synthetic exporter fixtures, not a broad CAD compatibility corpus.

Regression tests additionally cover meter and conversion-based inch units, source-byte preservation, Unicode/misleading-extension paths, typed EN/RU warnings, malformed/truncated Part 21 envelopes, empty geometry, lower input/memory/instance/source-triangle/rendered-triangle budgets, and pre-cancellation. Tiny WASM test modules exercise actual growth traps, running-loop interruption, deadlines, and controlled runtime traps without a GPU. Independent review identified ASCII-STL/STEP-extension precedence and weak entity-record framing; both were fixed and tested. An initial clock stub error was caught before STEP import succeeded and corrected to supply the real host timestamp.

A mutation experiment removed restoration of the STEP world offset. The distant-geometry regression failed on the expected minimum coordinate; the importer was restored byte-for-byte afterward.

A replacement-kernel smoke check decompressed the unmodified pinned crate WASM (23,023,739 bytes), selected it with `YO3MFS_STEP_KERNEL`, and successfully imported the distant box. A 32 MiB + 1 byte replacement was rejected with the typed kernel-size diagnostic. This verifies replacement loading, not a source rebuild.

A fresh optimized headless cylinder import including kernel initialization took 5.57 seconds; the hardened box check took 3.23 seconds. These are single local observations, not a general CAD, first-frame, FPS, or process-memory benchmark. Debug tests compile the CAD module more slowly. Initial fixed-kernel compilation is cached per process and is outside the 60-second conversion deadline; this limitation is documented.

The development package is at `target/step-validation/package/yo3mfs-1.4.0-arm64.dmg`. `hdiutil` emitted its existing creation-command deprecation warning but creation and verification succeeded. Hardened-runtime execution used an ad-hoc signature and does not prove Developer ID signing, Apple notarization, Gatekeeper acceptance on download, or Intel execution.

The initial native STEP check was blocked by the locked Mac. After the user unlocked it, the same packaged 1.4.0 ARM64 app was checked through its real window:

- The system picker opened `sample-cylinder.stp`; the viewport rendered the curved neutral mesh and the inspector showed 628 triangles, one combined instance, and rounded dimensions of 10.00 × 10.00 × 20.00 mm. The app reported 3.41 seconds for this first load.
- Orthographic/top views, wheel zoom, Fit all, keyboard orbit, and screen-space arrow panning visibly changed the view. Both persistent STEP conversion notes were available in Import notes.
- A deliberately malformed `invalid.step` produced the localized STEP error while retaining the cylinder and its statistics. Canceling the system picker also retained the scene. Existing errors and import notes changed immediately to Russian and back to English; the original English preference and background were restored/retained.
- Launching the packaged executable with `sample-assembly.step` as its file argument rendered both separated box placements. The inspector showed 24 triangles, one combined instance, and 50.00 × 20.00 × 30.00 mm; the app reported 3.44 seconds. Fixture hashes remained unchanged after the checks.

These checks establish local Apple Silicon window/picker/startup/rendering evidence, not full native acceptance. Physical drag-and-drop and cancellation during an actual CAD conversion were not exercised in this UI session. No new four-target GitHub run, native Intel/Windows/Linux GUI acceptance, real third-party STEP corpus, GPU-loss injection, library rebuild, or full distribution-license audit was completed. Published v1.3.1 assets remain unchanged.

## Local development launcher — October 19, 2026

Added executable `bin/test.dev` to build/run the optimized desktop app with the pinned toolchain and lockfile. The actual `bin/test.dev --version` build/run passed and printed `yo3mfs 1.4.0`. Shell syntax and focused launcher checks passed for invocation from another directory, relative/absolute model paths, spaces/Cyrillic, argument forwarding, and propagation of Cargo's nonzero exit status. README/SPEC local links and diff whitespace passed. This shell/documentation change did not rerun Rust suites or add new GUI/platform acceptance evidence.


## OBJ, glTF/GLB, and PLY — October 31, 2026

The unpublished 1.4.0 source now includes the [static mesh profile](MESH-FORMATS.md). This evidence is separate from the October 18 STL/STEP work and the published 1.3.1 release.

Completed local checks on macOS ARM64:

- Desktop/all-target/all-feature suite: **87 tests passed** (`rtk cargo test --locked --all-targets --all-features`).
- Core-only suite: **84 tests passed** (`rtk cargo test --locked --no-default-features`), including 20 tests across the new OBJ, PLY, glTF, and shared-format integration suites.
- Formatting and all-target/all-feature Clippy with warnings denied passed after review fixes.
- Packaging/release/signing regression suite: **23 tests passed**; these use mocked signing services and synthetic package executables and do not establish notarization or other-platform acceptance.
- Owned fixtures cover colored OBJ, ASCII/little-endian/big-endian PLY, embedded glTF, and GLB with two shared instances and a nested translation. Bounds, surface area, signed volume, instance/triangle counts, source immutability, provenance hashes, and generator reproducibility were checked.
- Tests cover concave polygon winding, negative/slash indices, unused-vertex bounds, PLY property ordering and ignored attributes, every binary PLY truncation, sparse/interleaved glTF data, normalized colors, reflections/scales, malformed/cyclic scenes, oversized references, required extensions, deformation rejection, memory/input/triangle/instance limits, and cancellation while reading.
- Independent review identified oversized glTF index narrowing, degenerate strip connectors, and fully skipped primitives breaking preparation. Regression tests reproduced each issue and passed after fixes. A mutation of the oversized-index guard made its regression test fail, then the original guard was restored.
- Both dictionaries compile exhaustively and retain placeholder-parity checks. Package notices include the added glTF/base64/parser-support license texts. Local documentation links and diff whitespace checks passed.

Additional completed checks:

- Optimized binary and inspector build passed: `rtk cargo build --locked --release --bins --example inspect`.
- Release binary `--version` returned `yo3mfs 1.4.0`; `--help` includes the new extensions. Release inspector smoke checks passed for 3MF, STL, OBJ, all three PLY encodings, embedded glTF, and GLB. OBJ/PLY tetrahedra measured 10 × 20 × 30 mm; the two glTF instances spanned 50 × 20 × 30 mm within float-input tolerance.
- Native macOS ARM64: opened the owned colored OBJ through the system picker in the newly built app. The window showed one instance, four triangles, 10 × 20 × 30 mm dimensions, and smoothly interpolated vertex colors. No shader/runtime failure appeared. The window changed during the next picker check; native PLY/glTF interaction and failed-load recovery were not marked complete. The app was left open.
- A local ad-hoc-signed development bundle/DMG was built under `target/format-validation/macos`. Disk-image creation required execution outside the filesystem sandbox; the sandboxed attempt reported `Device not configured`. Package verification passed: SHA-256 manifest, bundled notices/license files, strict ad-hoc code-signature verification, and DMG integrity. This is not a notarized public release.

Remaining acceptance: native macOS ARM64 PLY/glTF interaction, drag-and-drop and failed-load recovery for the new formats; independent real exporter/scanner files with recorded versions, macOS Intel native behavior, Windows native behavior, Linux X11 and Wayland native behavior, and public release signing/notarization. Synthetic fixtures are not exporter certification. Textures, external resources, animation, skins/morphs, and point clouds remain outside the documented static preview profile.


## Version 1.5.0 — native macOS follow-up, October 31, 2026

The owner requested completion of native PLY/glTF checks and a version/tag push. Version 1.5.0 combines the previously unpublished 1.4.0 STL/STEP implementation with OBJ, PLY, and glTF/GLB. Historical 1.4.0 evidence above describes development builds, not a published release.

Native window checks on the same Apple Silicon Mac used the optimized, ad-hoc-signed bundle under `target/format-validation/macos` (built as 1.4.0 before the metadata bump; the mesh import/UI implementation is unchanged, and the subsequent STEP runtime security update is recorded below):

- The real macOS picker opened ASCII, little-endian, and big-endian PLY. Each rendered interpolated vertex colors and showed one instance, four triangles, and 10.00 × 20.00 × 30.00 mm. Orthographic projection and Top view worked. Import notes exposed the persistent millimeter assumption.
- Embedded glTF and GLB each rendered both separated instances, showed eight triangles and 50.00 × 20.00 × 30.00 mm, and exposed the static-preview limitation note.
- An incomplete PLY without faces, a glTF referring to an external buffer, and a truncated GLB each showed the appropriate localized error while retaining the previous visible geometry, filename, dimensions, and counts. The external-buffer error advised exporting a self-contained file.
- Choosing another valid file through the error banner cleared the prior error and replaced the scene successfully.
- Finder drag attempts through the automation tool did not deliver a drop to the viewer. This is not evidence of an application failure or a successful automated drag-and-drop test. The owner subsequently completed the physical checks and reported success; see the confirmation below.

Release preparation: Cargo versions, README download targets, dependency notices, current format documentation, and release notes are synchronized to 1.5.0. The GitHub repository secret metadata was checked without retrieving values: **zero secrets are configured**. Tagged macOS signing and therefore public release publication will fail closed until the original Developer ID/notarization credentials are configured. No signing bypass or secret recovery was attempted.

Other limitations remain: independent exporter/scanner compatibility, native Intel Mac/Windows/Linux X11/Wayland GUI acceptance, clean-system/GPU-loss checks, and STEP corresponding-source/build-material distribution verification. Synthetic fixtures and passing automated builds do not establish these.

### Dependency audit during 1.5.0 preparation

The October 31 RustSec audit initially reported five Wasmtime advisories, including critical ARM64 `br_table` compilation advisory [RUSTSEC-2026-0379](https://rustsec.org/advisories/RUSTSEC-2026-0379). Wasmtime and its matching Cranelift/Pulley/internal crates were updated from 48.0.6 to [48.0.8](https://github.com/bytecodealliance/wasmtime/releases/tag/v48.0.8); the OCCT kernel and facade remain pinned at 5.0.1. The follow-up audit no longer reports Wasmtime vulnerabilities.

The raw audit is **not clean**: three zstd/zstd-safe vulnerability records and unsoundness notices remain in Wasmtime's optional persistent-cache dependency, enabled transitively by the upstream occt-wasm default feature set. The application creates its own Wasmtime engine from `OcctKernel::engine_config()` (which starts with `Config::new()`) and explicitly disables persistent caching with `cache(None)`; it caches only the already-compiled fixed module in process memory. The implicated zstd persistent-cache path is therefore unused by this host. An independent read-only review traced Wasmtime's absent-cache path directly to compilation, bypassing cache reads/writes and zstd processing, and confirmed that kernel decompression uses Brotli. No advisory was suppressed. Upstream feature separation or a patched compatible cache dependency remains follow-up work. Linux portal dependencies also retain three unmaintained-package notices; clipboard-win 5.4.1 was reported yanked by the online audit. These are not claims of a complete dependency or distribution-license audit.

The staged whitespace check reports trailing spaces in original OCCT-exported STEP fixtures and blank/trailing lines in copied upstream license texts. Those bytes are retained to preserve fixture hashes and source license copies. The check passes for authored source/documentation with these exact generated/upstream paths excluded.

### Final local 1.5.0 verification

After the Wasmtime update and explicit cache disable: formatting and Clippy (`--locked --all-targets --all-features -- -D warnings`) passed; **87 desktop/integration tests** and **84 core-only tests** passed. The optimized binary/inspector build, all ten format fixture smoke checks, 23 Python packaging/signing/release tests, actionlint, tag/version consistency, local documentation links, source fixture hashes, and mesh-generator reproducibility passed. The binary reports `yo3mfs 1.5.0`. A hardened-runtime ad-hoc-signed inspector with the audited entitlement imported the STEP box in 3.39 seconds with 12 triangles and 10 × 20 × 30 mm dimensions.

The final ad-hoc development package is `target/release-validation-1.5.0/macos/yo3mfs-1.5.0-arm64.dmg`, SHA-256 `4413124ba9f337b3bb1f5bf26329681a4ee8fd9f1ba2ee9d48f28b0e35250cd4`. Bundle version/icon/license checks, strict code-signature verification, SHA-256 manifest, and DMG integrity passed. Native launch of that exact bundle showed **Version 1.5.0** in About, then opened and rendered `sample.glb` through the picker with two instances, eight triangles, and 50 × 20 × 30 mm dimensions. This remains an ad-hoc local build, not notarization or other-platform acceptance.

### Owner-confirmed Finder drag-and-drop

On October 31, 2026, the owner manually dragged `sample-little.ply`, `sample.gltf`, and `sample.glb` from the prepared `target/native-formats` Finder window into YO3MFS, followed by `invalid.glb`. The owner reported: "All drops passed; invalid GLB retained the model." This closes the outstanding local macOS PLY/glTF/GLB physical-drop and dropped-file failure-recovery checks. It is owner-reported manual evidence, distinct from the automated picker checks above, and does not extend acceptance to other platforms or exporters.

The implementation commit `7203c35c711c5d1356a3345c7e97efcff136a6b2` and annotated `v1.5.0` tag were pushed to GitHub. This documentation follow-up leaves that tag unchanged. Package publication remains subject to the four-target workflow and configured macOS signing credentials.

### First GitHub 1.5.0 runs

The [tag run](https://github.com/mpakus/yo3mfs/actions/runs/41240711117) completed with Linux passing and both macOS jobs reaching signing, then failing closed because all seven required signing secrets were absent. Windows failed the malformed-STEP integration test. The [branch run](https://github.com/mpakus/yo3mfs/actions/runs/41240711076) passed both macOS jobs and Linux; Windows had the same test failure. No release was published.

### Windows fixture-test correction

The Windows failure was reproduced locally with CRLF input: the test's `ENDSEC;\nEND-ISO` replacement did not match, leaving a valid STEP file unchanged. A regression assertion failed on that no-op before the fix. The malformed record is now inserted after the `DATA;` token, independently of line endings. All malformed variants are exercised with LF and CRLF, and each must differ from the valid input before rejection is checked. No importer behavior or acceptance limit was weakened.

`.gitattributes` disables text conversion only for `tests/fixtures/**`, preserving the authored/exported bytes and provenance hashes. All 13 manifest fixture hashes were checked through Git's checkout filter with `core.autocrlf=true` and remained identical. The correction is a follow-up on `main`; the existing `v1.5.0` tag remains at its original commit and retains its original failed Windows run. A future release containing this correction needs a new version tag.

Local verification of the test correction: all six STEP integration tests passed with `cargo test --locked --no-default-features --test import_step`; formatting, all-target/all-feature Clippy with warnings denied, and diff checks passed. The new Windows hosted run is separate validation and is not claimed by these local results.
