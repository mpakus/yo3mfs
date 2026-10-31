# YO3MFS portable package

YO3MFS is an offline 3MF, STL, STEP, OBJ, glTF/GLB, and PLY viewer for macOS, Windows, and Linux. It displays geometry and saved Bambu Studio colors without changing or uploading your file.

## Start

- macOS: open the DMG, copy `yo3mfs.app` to Applications, and open it.
- Windows: extract the ZIP and run `yo3mfs.exe`.
- Linux: extract the tar.gz and run `./yo3mfs` in a graphical session. The package was built on Ubuntu 24.04 x64. System graphics libraries and `xdg-desktop-portal` with your desktop's portal backend are required; they are not bundled.

Choose File → Open or drop one `.3mf`, `.stl`, `.step`, `.stp`, `.obj`, `.gltf`, `.glb`, or `.ply` into the window. A path can also be passed to the executable. Bambu Studio does not need to be installed.

STL accepts ASCII and binary geometry. Its unitless coordinates are interpreted as millimeters, with a notice in Import notes. Nonstandard STL colors/attributes are not applied; the model is shown in neutral material. Multiple ASCII solids retain their original positions in one mesh.

STEP/STP previews embedded CAD geometry in millimeters, with tessellated curves and original assembly placements combined into one neutral mesh. Colors, names, PMI, and external files are not loaded. Import notes disclose topology normalization/healing and possible omitted entities. Compare the preview with your CAD source. The first STEP load initializes the CAD engine in the background; input is limited to 64 MiB and conversion to 60 seconds after initialization.

OBJ/PLY/glTF are static previews: embedded vertex/base colors are supported; external buffers, MTL/textures, animation, skins/morphs, and point clouds are not loaded. Export self-contained GLB for glTF geometry. OBJ/PLY coordinates are interpreted as millimeters with a notice.

### macOS verification warning

Development packages and the older published v1.3.1 are not notarized. Tagged v1.5.0 macOS packages must pass Developer ID signing and notarization before publication. For an unnotarized development package: If you trust the build downloaded from this repository's Releases, copy the app to Applications and try opening it. When macOS cannot verify it, choose **Done**, then open **System Settings → Privacy & Security → Open Anyway** for YO3MFS. Confirm **Open** and authenticate if requested. This creates a local exception for this app. [Apple's instructions](https://support.apple.com/en-us/102445).

## Controls

- Left drag: rotate. Scroll or `+` / `−`: zoom.
- Click the model area, then use arrows to pan; Shift moves faster and Alt + arrows rotates.
- `F`: fit all. `Home`: reset view. `⌘O` / `Ctrl+O`: open.
- Materials → Project colors / Neutral changes the display mode.
- Settings changes language; English and Russian are available.
- YO3MFS → About shows version and credits. Help → Controls opens the guide.

Background, language, and color mode are saved locally. The last model path is not saved. The viewer does not slice, print, paint new colors, or export models. Coincident surfaces in overlapping parts may flicker because slicer Boolean operations are not performed.

## Package status

Download versioned packages and SHA-256 checksums from [GitHub Releases](https://github.com/mpakus/yo3mfs/releases). Version 1.5.0 source adds STL, STEP, OBJ, PLY, and glTF/GLB; publication and native acceptance are separate. Development macOS builds are ad-hoc signed, while tagged releases require notarization. Windows Authenticode signing is not configured. A successful build does not replace native GUI/clean-system testing. The project license has not yet been selected; see the repository for distribution status.

STEP uses a sandboxed Open CASCADE kernel. `THIRD-PARTY-NOTICES.txt` and `licenses/` contain its terms and source information; `YO3MFS_STEP_KERNEL` selects a compatible replacement before launch. See the [STEP profile and rebuild instructions](https://github.com/mpakus/yo3mfs/blob/main/docs/STEP.md).

The bundled font license is `NotoSansSymbols2-OFL.txt` (inside `Contents/Resources` on macOS). Linux includes `share/applications/com.yo3mfs.viewer.desktop` and `share/icons/hicolor/512x512/apps/com.yo3mfs.viewer.png`; user-local launcher installation is documented in the repository README.

[Source, full instructions, compatibility, and validation](https://github.com/mpakus/yo3mfs)

[Made in Austin ✩ Texas](https://aomega.co/)
