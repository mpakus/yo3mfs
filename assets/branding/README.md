# YO3MFS — утверждённый брендинг

Владелец одобрил логотип и иконку в чате 16 октября 2026 года: «отличное лого и иконка, делаем».

- [Иконка PNG](yo3mfs-icon.png) — исходный знак с настоящей прозрачностью вокруг графитовой плитки.
- [Логотип для тёмного интерфейса](yo3mfs-logo-dark.png) — светлая надпись YO3MFS, медная Y и графитовый фон для About. Сплошной фон выбран, чтобы сохранить чистые края светлых букв.
- [ICNS](yo3mfs.icns) — macOS Finder/Dock; PNG-представления до 1024 px.
- [ICO](yo3mfs.ico) — Windows, семь размеров от 16 до 256 px, встраивается в EXE через build.rs.
- `icons/` — PNG 16–1024 px; 256 px встроен в окно и пустое состояние, 512 px включён в Linux-пакет.
- [Исходный обзорный макет](concepts/yo3mfs-identity-v1.png) сохранён без изменений.
- Мастер логотипа сохранён в `sources/`; встроенная версия уменьшена до 1200 × 400 для About шириной 300 логических пикселей и масштаба до 4x. Это также сохраняет совместимость с минимальным пределом текстур egui 2048 px.

Изображения подготовлены встроенным image_gen в режиме редактирования утверждённого макета, не CLI. Первый прозрачный вариант светлого логотипа отброшен из-за артефактов альфа-канала и не включён в проект. Промпты принятых файлов приведены ниже.

Размеры и контейнеры повторно создаются на macOS командой `python3 scripts/build_icons.py`: sips выполняет только масштабирование, iconutil создаёт ICNS, Python собирает ICO из готовых PNG. Исходная графика не перерисовывается. Обычная сборка на любой целевой ОС использует сохранённые файлы и не требует генератора изображений.

Окно получает IconData, логотип и иконка загружаются в GPU один раз. macOS-пакет ссылается на ICNS через Info.plist. Windows-ресурс собирается [winresource](https://docs.rs/winresource/latest/winresource/) с Windows SDK; при cross-check на macOS можно указать `RC_PATH` для llvm-rc. Linux app_id и имя desktop entry совпадают: `com.yo3mfs.viewer`. Установка desktop entry и PNG в пользовательские каталоги описана в README; приложение само не меняет системные ассоциации.

## Prompt: иконка

Use case: background-extraction. Asset type: production desktop app icon PNG. Input image: approved YO3MFS brand board, edit target. Extract ONLY the large APP ICON on the right, faithfully preserving its exact graphite rounded square tile, subtle bevel, cube geometry and copper-peach Y mark. This is final production extraction, not a redesign: match the approved shape, proportions, colors and lighting. Remove all surrounding board, captions, other marks and shadows cast onto the board. Single centered front-facing square icon on genuinely transparent canvas. Square image, icon tile occupies about 88% of width and height with symmetrical transparent padding; do not crop its rounded corners. No wordmark, no labels, no additional elements. Preserve actual alpha transparency outside the tile, no simulated checkerboard.

## Prompt: логотип

Use case: precise-object-edit. Asset type: final YO3MFS horizontal logo for an About window. Use the approved reference board's LEFT horizontal logo only. Preserve the exact geometric cube with copper-peach structural Y and the exact typography/spacing of YO3MFS (Y O numeral 3 M F S). Render the wordmark in solid warm white #F2F4F7 and cube dark faces in cool slate #687481 for dark mode. Important: the entire background must be perfectly flat opaque #22262D edge to edge, including inside the O and between letters; absolutely NO transparency, no speckles, no texture, no distress, no bright stray pixels, no drop shadow. Clean solid smooth vector-like contours, precise polished professional logo. Wide 3:1 canvas, one horizontal logo filling about 90% width centered both axes. Keep copper Y unchanged. No captions, additional icons, frames, or other text.

