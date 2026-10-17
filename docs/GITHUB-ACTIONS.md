# GitHub builds and releases

The [Desktop checks and packages workflow](../.github/workflows/ci.yml) builds YO3MFS on four native GitHub-hosted runners:

| Job | Runner | Rust target | Downloadable package |
| --- | --- | --- | --- |
| macOS Apple Silicon | `macos-15` | `aarch64-apple-darwin` | `yo3mfs-VERSION-aarch64-apple-darwin.dmg` |
| macOS Intel | `macos-15-intel` | `x86_64-apple-darwin` | `yo3mfs-VERSION-x86_64-apple-darwin.dmg` |
| Windows x64 | `windows-2025` | `x86_64-pc-windows-msvc` | `yo3mfs-VERSION-x86_64-pc-windows-msvc.zip` |
| Linux x64 | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `yo3mfs-VERSION-x86_64-unknown-linux-gnu.tar.gz` |

Runner labels/architectures were checked against the [official runner matrix](https://github.com/actions/runner-images) and [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Windows and Linux ARM64 are outside the current build matrix.

## Download a release

Open [Releases](https://github.com/mpakus/yo3mfs/releases/latest), choose the package for your platform, and download `SHA256SUMS` to verify it. The [README](../README.md#download-and-run) also links directly to each package for the current version. Release assets remain available independently of the 14-day Actions artifact retention period.

Branch pushes, pull requests, and manual branch runs produce development artifacts without publishing a Release. Open a successful [Actions run](https://github.com/mpakus/yo3mfs/actions/workflows/ci.yml) and download `yo3mfs-macos-arm64`, `yo3mfs-macos-x64`, `yo3mfs-windows-x64`, or `yo3mfs-linux-x64`. Extract the wrapper ZIP to find the native package and its single-package checksum file.

## Publish a version

The owner authorized version-tag publishing for v1.3.1 and subsequently requested reuse of GitRonimo's signing setup. GitHub publication uses its scoped `GITHUB_TOKEN`; new macOS release builds additionally require the seven [signing and notarization secrets](MACOS-SIGNING.md). Configure them before pushing a new version tag. Existing v1.3.1 assets remain unnotarized.

1. Update the package version in `Cargo.toml` and the `yo3mfs` entry in `Cargo.lock` together.
2. Add English notes at `docs/releases/vVERSION.md`, and update the current version and all direct package URLs in README. Keep historical validation records unchanged.
3. Commit the reviewed change to `main`, push it, and push an annotated tag for exactly that commit. For 1.3.1:

   ```bash
   git push origin main
   git tag -a v1.3.1 -m "YO3MFS v1.3.1"
   git push origin v1.3.1
   ```

4. Watch the tag's Actions run. A release tag must be exactly `vMAJOR.MINOR.PATCH`, match both Cargo files, and have release notes. All four native jobs must pass before **Publish GitHub Release** starts.
5. The release job downloads only artifacts from that same run, requires all four platforms, checks their versioned filenames and SHA-256 hashes, and combines the four checksums. It creates a draft, uploads four archives plus `SHA256SUMS`, then publishes the release as Latest.
6. Open the public release and verify all five assets and the README links. Build success does not replace native GUI acceptance.

The workflow can also be dispatched against an existing version tag, for example `gh workflow run ci.yml --ref v1.3.1`. An interrupted upload leaves a draft that a rerun can resume. Published releases are never overwritten: a rerun stops if the release is already public. Fix a published build with a new patch version; do not move its tag or replace its assets. If a release must be withdrawn, the owner can remove it from Latest or delete the affected release after reviewing the impact; retain source history and use a new version for the correction.

Implementation: [release.py](../scripts/release.py). Publication uses the official [GitHub CLI release commands](https://cli.github.com/manual/gh_release_create) and a pinned [download-artifact action](https://github.com/actions/download-artifact).

## Build gates

Every matrix job checks out source without persisted Git credentials, installs Python 3.13, installs the Rust version/components from `rust-toolchain.toml`, and uses `Cargo.lock`. Linux installs its native build libraries. Cargo registry, Git dependencies, and target output are cached separately by target/toolchain/lockfile. On a cache miss, a previous cache for the same target can be restored; Cargo rebuilds changed inputs and every check still runs.

The job then runs:

1. `cargo fmt --all -- --check` and Python packaging/release regression tests.
2. Clippy for all targets/features with `-D warnings`.
3. Desktop/integration tests and separate core tests without GUI dependencies.
4. An optimized release build for the explicit target triple.
5. A `--version` smoke check against the version in `Cargo.toml`.
6. Native packaging with `scripts/package.py`.
7. Tagged macOS builds sign and notarize their app/DMG using `scripts/sign_macos.py`, staple the ticket, check Gatekeeper, and regenerate checksums. Credential files and the temporary keychain are cleaned up, including after failure.
8. `scripts/verify_package.py`: checksum and package structure; macOS also verifies the bundle signature and DMG, Linux checks executable permissions. Tagged macOS builds use `--require-notarization` to independently check Developer ID, hardened runtime, the stapled ticket, and Gatekeeper.
9. Artifact upload only after all checks pass.

Actions are pinned to verified release commit SHAs, with version comments. Default permissions and build jobs are `contents: read`; only the tag publishing job receives `contents: write`. There is no `pull_request_target` or cross-run artifact promotion. Build jobs have a 60-minute timeout and do not fail-fast across platforms. Newer branch/PR runs cancel superseded runs; tag builds are not canceled this way. No private 3MF files or optional external compatibility models are downloaded in CI.

## Checksums and local reproduction

For downloaded packages on Linux (skip entries for other platforms):

```bash
sha256sum --check --ignore-missing SHA256SUMS
```

On macOS, compare the result with the matching entry in `SHA256SUMS`:

```bash
shasum -a 256 yo3mfs-VERSION-aarch64-apple-darwin.dmg
```

On Windows PowerShell, compare the result with `SHA256SUMS`:

```powershell
Get-FileHash .\yo3mfs-VERSION-x86_64-pc-windows-msvc.zip -Algorithm SHA256
```

For local packaging, use Python 3.11+ and a clean output directory. Example for Apple Silicon:

```bash
cargo build --locked --release --target aarch64-apple-darwin
python3 scripts/package.py --target aarch64-apple-darwin --dmg --output-dir dist/local-arm64
python3 scripts/verify_package.py dist/local-arm64
```

On Windows/Linux, substitute the native target and omit `--dmg`. The verifier expects exactly one archive and its checksum in the directory. A macOS bundle without `--dmg` is a local `.app`, not an archive accepted by this verifier.

## What a successful build does not prove

CI tests do not open a native GUI or exercise drag-and-drop, trackpads, physical GPUs, or Linux portals. Those remain manual acceptance checks on every target. Linux packages are built on Ubuntu 24.04 and do not promise compatibility with every distribution or older glibc.

Published v1.3.1 and development macOS packages have ad-hoc signatures. The new tag workflow requires Developer ID signing and notarization; a credentialed run is still needed to validate that path. Windows packages are not Authenticode-signed. License selection/dependency review and clean-system installation checks remain outstanding. Publishing a downloadable release does not claim those checks are complete or grant an open-source license.
