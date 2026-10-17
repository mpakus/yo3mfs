# YO3MFS portable package

YO3MFS is an offline 3MF viewer for macOS, Windows, and Linux. It displays geometry and saved Bambu Studio colors without changing or uploading your file.

## Start

- macOS: open the DMG, copy `yo3mfs.app` to Applications, and open it.
- Windows: extract the ZIP and run `yo3mfs.exe`.
- Linux: extract the tar.gz and run `./yo3mfs` in a graphical session. The package was built on Ubuntu 24.04 x64. System graphics libraries and `xdg-desktop-portal` with your desktop's portal backend are required; they are not bundled.

Choose File → Open or drop one `.3mf` into the window. A path can also be passed to the executable. Bambu Studio does not need to be installed.

### macOS verification warning

The current macOS package is not notarized. If you trust the build downloaded from this repository's Releases, copy the app to Applications and try opening it. When macOS cannot verify it, choose **Done**, then open **System Settings → Privacy & Security → Open Anyway** for YO3MFS. Confirm **Open** and authenticate if requested. This creates a local exception for this app. [Apple's instructions](https://support.apple.com/en-us/102445).

## Controls

- Left drag: rotate. Scroll or `+` / `−`: zoom.
- Click the model area, then use arrows to pan; Shift moves faster and Alt + arrows rotates.
- `F`: fit all. `Home`: reset view. `⌘O` / `Ctrl+O`: open.
- Materials → Project colors / Neutral changes the display mode.
- Settings changes language; English and Russian are available.
- YO3MFS → About shows version and credits. Help → Controls opens the guide.

Background, language, and color mode are saved locally. The last model path is not saved. The viewer does not slice, print, paint new colors, or export models. Coincident surfaces in overlapping parts may flicker because slicer Boolean operations are not performed.

## Package status

Download versioned packages and SHA-256 checksums from [GitHub Releases](https://github.com/mpakus/yo3mfs/releases). macOS uses an ad-hoc signature without Developer ID/notarization; Windows Authenticode signing is not configured. A successful build does not replace native GUI/clean-system testing. The project license has not yet been selected; see the repository for distribution status.

The bundled font license is `NotoSansSymbols2-OFL.txt` (inside `Contents/Resources` on macOS). Linux includes `share/applications/com.yo3mfs.viewer.desktop` and `share/icons/hicolor/512x512/apps/com.yo3mfs.viewer.png`; user-local launcher installation is documented in the repository README.

[Source, full instructions, compatibility, and validation](https://github.com/mpakus/yo3mfs)

[Made in Austin ✩ Texas](https://aomega.co/)
