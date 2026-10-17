# macOS signing and notarization

YO3MFS adapts the Developer ID / App Store Connect API-key setup from [GitRonimo's release workflow](https://github.com/mpakus/gitronimo/blob/fbf250ea3815e3157e28e90ca1b131e1e546c8a1/.github/workflows/release.yml). Its signed v2.0.6 release run passed. Cuckoding's current desktop configuration uses local ad-hoc signing; KickstandMoto uses iOS/EAS profiles, which are not macOS Developer ID credentials.

## Current status

The published YO3MFS **v1.3.1** DMGs are still ad-hoc signed and unnotarized. Adding this workflow does not change those assets. Signed distribution requires the secrets below, a new patch version/tag, successful Apple notarization, and verification of the downloaded packages. Do not replace v1.3.1 assets or move its tag.

Branch and pull-request builds remain ad-hoc signed. A `v*` tag requires signing and notarization on both macOS jobs; missing credentials, failed signing, rejection, timeouts, or failed Gatekeeper checks prevent publishing the entire release. Credentials are exposed only to the signing step on tag builds.

## Repository secrets

Use the same names and credential types as GitRonimo, configured on `mpakus/yo3mfs`:

| Secret | Value |
| --- | --- |
| `DEVELOPER_ID_APPLICATION` | Full Developer ID Application certificate identity |
| `MACOS_CERTIFICATE_BASE64` | Base64-encoded PKCS#12 (`.p12`) containing the certificate and its private key |
| `MACOS_CERTIFICATE_PASSWORD` | Password protecting that `.p12` |
| `KEYCHAIN_PASSWORD` | Password for the temporary CI keychain; a new random value can be used for YO3MFS |
| `APPLE_API_KEY_BASE64` | Base64-encoded App Store Connect API private key (`.p8`) authorized for notarization |
| `APPLE_API_KEY_ID` | API key identifier |
| `APPLE_API_ISSUER` | API key issuer identifier |

GitHub can list secret names but cannot return their stored values. Reuse the original signing files/configuration, or have the owner add the values through **Settings → Secrets and variables → Actions**. Do not retrieve them through workflow logs or artifacts. An iOS distribution certificate or provisioning profile cannot replace a Developer ID Application certificate.

With the original files outside the repository, upload encoded files through standard input (not command arguments):

```bash
base64 < /secure/path/developer-id.p12 | tr -d '\n' | gh secret set MACOS_CERTIFICATE_BASE64 --repo mpakus/yo3mfs
base64 < /secure/path/AuthKey.p8 | tr -d '\n' | gh secret set APPLE_API_KEY_BASE64 --repo mpakus/yo3mfs
```

For each remaining value, `gh secret set SECRET_NAME --repo mpakus/yo3mfs` prompts for input. Never put passwords/private keys into source files, chat, shell history, or release notes. Existing local Developer ID signing ability does not prove that an exportable `.p12` and usable notarization API key are available for hosted runners.

## Release sequence

After ordinary build/tests and native packaging, [sign_macos.py](../scripts/sign_macos.py):

1. Validates the required configuration and creates a private directory under `RUNNER_TEMP`.
2. Imports the certificate into an isolated temporary keychain. It does not replace the default user keychain or its search list.
3. Signs the app with Developer ID, hardened runtime, and a secure timestamp, then rebuilds and signs the DMG.
4. Submits that DMG once to Apple's notary service. It waits for the same submission for up to three 15-minute periods. Only explicit `Accepted` proceeds; a rejected submission fails immediately.
5. Staples and validates the DMG ticket, then checks Gatekeeper acceptance of both the DMG and app. Apple notarizes the signed app inside the submitted DMG as well as its container.
6. Removes the keychain and credential files, then regenerates `SHA256SUMS` from the final stapled DMG. An `always()` cleanup step also runs after signing failure/cancellation.

[verify_package.py](../scripts/verify_package.py) runs with `--require-notarization` for tagged macOS packages. It independently requires Developer ID/hardened runtime, validates the DMG signature and ticket, checks Gatekeeper, and verifies the checksum. The existing publishing job still waits for all four platforms before uploading any public assets.

The signing script keeps secret-bearing subprocess commands out of error messages. Never enable shell tracing or print the signing environment. Temporary credentials are excluded from artifacts and the Cargo cache.

## Verification

Unit tests cover missing/invalid configuration, redaction, cleanup after failure, waiting without duplicate uploads, Apple rejection/unknown status, and preservation of the intermediate checksum on failure. These mocked tests do not prove a real certificate import or Apple acceptance.

After a credentialed release, download both DMGs, verify SHA-256, validate their stapled tickets, and mount them read-only to inspect the contained app signature/version and run Gatekeeper assessment. A local non-quarantined build is not a first-download test. Record the actual run, notarization results, and native acceptance in [VALIDATION.md](VALIDATION.md).

References: [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [packaging Mac software](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution), [GitHub certificate import guidance](https://docs.github.com/en/actions/how-tos/deploy/deploy-to-third-party-platforms/sign-xcode-applications).
