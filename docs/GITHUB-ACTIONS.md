# GitHub builds

The [Desktop checks and packages workflow](../.github/workflows/ci.yml) builds YO3MFS on four native GitHub-hosted runners:

| Job | Runner | Rust target | Downloadable package |
| --- | --- | --- | --- |
| macOS Apple Silicon | `macos-15` | `aarch64-apple-darwin` | `yo3mfs-VERSION-aarch64-apple-darwin.dmg` |
| macOS Intel | `macos-15-intel` | `x86_64-apple-darwin` | `yo3mfs-VERSION-x86_64-apple-darwin.dmg` |
| Windows x64 | `windows-2025` | `x86_64-pc-windows-msvc` | `yo3mfs-VERSION-x86_64-pc-windows-msvc.zip` |
| Linux x64 | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `yo3mfs-VERSION-x86_64-unknown-linux-gnu.tar.gz` |

Runner labels/architectures were checked against the [official runner matrix](https://github.com/actions/runner-images) and [GitHub-hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Windows and Linux ARM64 are outside the current build matrix.

## First run

1. Push the project, including `.github/workflows/ci.yml`, `Cargo.lock`, `rust-toolchain.toml`, scripts, and committed assets, to [mpakus/yo3mfs](https://github.com/mpakus/yo3mfs). No signing secrets are required.
2. Open [Actions](https://github.com/mpakus/yo3mfs/actions/workflows/ci.yml). A push or pull request starts the workflow automatically. After the workflow exists on the default branch, Run workflow can also start a manual run.
3. Open a successful run and download its Artifacts: `yo3mfs-macos-arm64`, `yo3mfs-macos-x64`, `yo3mfs-windows-x64`, or `yo3mfs-linux-x64`.
4. Extract the Actions artifact wrapper. Inside are the platform package and `SHA256SUMS`. Artifacts expire after 14 days.

This change configures builds; it does not push source, create a tag, select a license, or publish a GitHub Release. As of the local verification, the remote repository is empty and has no Actions runs. Record actual remote results in [VALIDATION.md](VALIDATION.md) after the first push. GitHub artifacts and public Releases are different distribution mechanisms.

## Build gates

Every matrix job checks out source without persisted Git credentials, installs Python 3.13, installs the Rust version/components from `rust-toolchain.toml`, and uses `Cargo.lock`. Linux installs its native build libraries. Cargo registry, Git dependencies, and target output are cached separately by target/toolchain/lockfile.

The job then runs:

1. `cargo fmt --all -- --check` and Python packaging regression tests.
2. Clippy for all targets/features with `-D warnings`.
3. Desktop/integration tests and separate core tests without GUI dependencies.
4. An optimized release build for the explicit target triple.
5. A `--version` smoke check against the version in `Cargo.toml`.
6. Native packaging with `scripts/package.py`.
7. `scripts/verify_package.py`: checksum and package structure; macOS also verifies the bundle signature and DMG, Linux checks executable permissions.
8. Artifact upload only after all checks pass.

Actions are pinned to verified release commit SHAs, with version comments. Repository permissions are `contents: read`; artifact upload does not require release write access. Jobs have a 60-minute timeout, do not fail-fast across platforms, and newer runs cancel superseded runs for the same ref/PR. No private 3MF files or optional external compatibility models are downloaded in CI.

## Checksums and local reproduction

For a downloaded package on Linux:

```bash
sha256sum --check SHA256SUMS
```

On macOS:

```bash
shasum -a 256 -c SHA256SUMS
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

macOS packages have ad-hoc signatures, without Developer ID/notarization. Windows packages are not Authenticode-signed. Public-release signing, license selection/dependency review, clean-system installation, and a release-publication workflow remain separate decisions. The current workflow does not publish releases automatically or request signing secrets.
