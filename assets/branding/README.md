# YO3MFS — approved branding

The owner approved the logo and icon in the conversation on October 16, 2026 and authorized their use in the application.

- [PNG icon](yo3mfs-icon.png): the source mark with actual transparency around the graphite tile.
- [Dark-interface logo](yo3mfs-logo-dark.png): light YO3MFS lettering, copper Y, and a graphite background for About. An opaque background preserves clean edges around the light letters.
- [ICNS](yo3mfs.icns): macOS Finder/Dock, with PNG representations up to 1024 px.
- [ICO](yo3mfs.ico): seven sizes from 16 to 256 px, embedded in the Windows EXE by `build.rs`.
- `icons/`: PNGs from 16 to 1024 px; 256 px is embedded in the window/empty state, and 512 px is included in Linux packages.
- The [original concept board](concepts/yo3mfs-identity-v1.png) is preserved unchanged.
- The logo master is retained in `sources/`; the embedded version is reduced to 1200 × 400 for an About logo 300 logical pixels wide at up to 4× scale. This also fits egui's minimum 2048 px texture limit.

The assets were prepared with the built-in image_gen editor using the approved concept, not a CLI. The first transparent light-logo attempt was rejected because of alpha artifacts and is not included. Prompts for the accepted assets follow below.

Run `python3 scripts/build_icons.py` on macOS to regenerate sizes/containers: sips only resizes, iconutil creates ICNS, and Python assembles ICO from PNGs. It does not redraw the artwork. Normal builds on every target OS use committed assets and do not need image generation.

The window receives IconData; logo/icon textures load once. The macOS bundle references ICNS through Info.plist. Windows resources use [winresource](https://docs.rs/winresource/latest/winresource/) with the Windows SDK; macOS cross-checks may set `RC_PATH` to llvm-rc. The Linux app ID matches its desktop entry: `com.yo3mfs.viewer`. README explains user-local desktop-entry/PNG installation; the application does not change system file associations.

## Prompt: icon

Use case: background-extraction. Asset type: production desktop app icon PNG. Input image: approved YO3MFS brand board, edit target. Extract ONLY the large APP ICON on the right, faithfully preserving its exact graphite rounded square tile, subtle bevel, cube geometry and copper-peach Y mark. This is final production extraction, not a redesign: match the approved shape, proportions, colors and lighting. Remove all surrounding board, captions, other marks and shadows cast onto the board. Single centered front-facing square icon on genuinely transparent canvas. Square image, icon tile occupies about 88% of width and height with symmetrical transparent padding; do not crop its rounded corners. No wordmark, no labels, no additional elements. Preserve actual alpha transparency outside the tile, no simulated checkerboard.

## Prompt: logo

Use case: precise-object-edit. Asset type: final YO3MFS horizontal logo for an About window. Use the approved reference board's LEFT horizontal logo only. Preserve the exact geometric cube with copper-peach structural Y and the exact typography/spacing of YO3MFS (Y O numeral 3 M F S). Render the wordmark in solid warm white #F2F4F7 and cube dark faces in cool slate #687481 for dark mode. Important: the entire background must be perfectly flat opaque #22262D edge to edge, including inside the O and between letters; absolutely NO transparency, no speckles, no texture, no distress, no bright stray pixels, no drop shadow. Clean solid smooth vector-like contours, precise polished professional logo. Wide 3:1 canvas, one horizontal logo filling about 90% width centered both axes. Keep copper Y unchanged. No captions, additional icons, frames, or other text.

