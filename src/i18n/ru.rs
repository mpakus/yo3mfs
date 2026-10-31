use super::Key;

pub(super) const fn get(key: Key) -> &'static str {
    match key {
        Key::EmbeddedColors => {
            "Показаны встроенные цвета сетки. Палитры филаментов в этой модели нет."
        }
        Key::InvalidModel => "Некорректная модель: {0}",
        Key::InvalidObj => "Некорректный OBJ: {0}",
        Key::InvalidPly => "Некорректный PLY: {0}",
        Key::InvalidGltf => "Некорректный glTF/GLB: {0}",
        Key::MeshSyntax => "Некорректные или неполные данные сетки.",
        Key::MeshPolygon => {
            "Грань вырождена, не лежит в одной плоскости, пересекает себя или не может быть разбита на треугольники."
        }
        Key::MeshLineSize => "длина строки сетки (не более 64 КиБ)",
        Key::MeshFaceSize => "число вершин многоугольника (не более 1 024)",
        Key::MeshUnitsWarning => {
            "В этом формате нет надёжных данных о единицах измерения. Координаты интерпретируются в миллиметрах."
        }
        Key::ObjMaterialsWarning => {
            "Материалы и текстуры OBJ не загружаются; внешние файлы MTL не открываются. Встроенные цвета вершин отображаются при наличии."
        }
        Key::MeshAttributesWarning => {
            "Дополнительные атрибуты, сохранённые нормали и данные вне поверхности не отображаются. Освещение рассчитывается по геометрии граней."
        }
        Key::MeshUnsupported => {
            "Эта геометрия не поддерживается статическим просмотром сетки. Экспортируйте полигональную сетку."
        }
        Key::PlyPoints => {
            "PLY содержит точки без граней. Для просмотра экспортируйте сетку поверхности."
        }
        Key::GltfExternal => {
            "Внешние буферы glTF не открываются. Экспортируйте самодостаточный GLB или glTF со встроенными буферами."
        }
        Key::GltfPreviewWarning => {
            "Статический просмотр glTF: базовые цвета показаны с простым освещением. Текстуры, прозрачность, эффекты PBR, камеры и анимация не воспроизводятся."
        }
        Key::GltfSceneWarning => {
            "Показана только сцена по умолчанию (или первая сцена, если сцена по умолчанию не задана)."
        }
        Key::GltfDeformation => {
            "Скелетная деформация и морфинг не поддерживаются. Экспортируйте статическую сетку с применённой деформацией."
        }
        Key::GltfInputSize => {
            "размер glTF или декодированных буферов (не более 64 МиБ; JSON — 16 МиБ)"
        }
        Key::GltfStructure => "Некорректная сцена, буфер, аксессор или преобразование glTF.",
        Key::GltfExtensionsWarning => {
            "Необязательные расширения glTF не применяются; базовый просмотр сетки может отличаться."
        }

        Key::StepFaces => "число граней STEP (не более 100 000)",
        Key::StepKernelSize => "Размер заменяющего ядра STEP превышает 32 МиБ.",
        Key::StepKernelTrap => "CAD-ядро STEP неожиданно остановилось. Попробуйте другую модель.",
        Key::InvalidStep => "Некорректный STEP: {0}",
        Key::StepFormat => {
            "Ожидался полный файл STEP Part 21 (.step или .stp). Сжатый STEP и внешние файлы моделей не поддерживаются."
        }
        Key::StepInputSize => "размер файла STEP (не более 64 МиБ)",
        Key::StepMemory => "память для преобразования STEP",
        Key::StepTimeout => "Преобразование STEP превысило ограничение в 60 секунд.",
        Key::StepKernel => "Ошибка CAD-ядра STEP: {0}",
        Key::StepIncomplete => {
            "STEP содержит отсутствующие, неоднозначные, некорректные или неразбитые на треугольники грани."
        }
        Key::StepPreviewWarning => {
            "Предпросмотр STEP: криволинейные поверхности разбиты на треугольники в миллиметрах, сборки объединены. Цвета, имена, PMI и внешние файлы не загружаются."
        }
        Key::StepConversionWarning => {
            "CAD-ядро может нормализовать или исправлять топологию STEP и пропускать неподдерживаемые элементы. Полноту переноса корневых объектов проверить нельзя; сравните результат с исходной CAD-моделью."
        }
        Key::InvalidStl => "Некорректный STL: {0}",
        Key::StlFormat => {
            "Ожидался текстовый STL или двоичный STL с корректным числом граней. Откройте модель 3MF, STL, STEP, OBJ, glTF/GLB или PLY."
        }
        Key::StlLength => "Двоичный STL обрезан или содержит данные сверх указанного числа граней.",
        Key::StlSyntax => "Ошибка структуры текстового STL около строки {0}.",
        Key::StlNumber => "Нормали и координаты STL должны быть конечными числами.",
        Key::StlLineSize => "длина строки STL",
        Key::InputSize => "размер входного файла",
        Key::StlUnitsWarning => {
            "STL не содержит единиц измерения. Координаты интерпретируются в миллиметрах."
        }
        Key::StlAttributesWarning => {
            "Цвета и атрибуты STL не применены. Геометрия показана нейтральным материалом."
        }
        Key::CliHelp => {
            "yo3mfs {0}\nИспользование: yo3mfs [path/to/model.3mf | path/to/model.stl | path/to/model.step | path/to/model.obj | path/to/model.glb | path/to/model.gltf | path/to/model.ply]\nАвтономный просмотрщик 3MF, STL, STEP, OBJ, glTF/GLB и PLY. Исходные файлы не изменяются."
        }
        Key::BadPaint => "Некорректная раскраска грани Bambu",
        Key::PaintDepth => "глубина разбиения раскраски",
        Key::ProjectSettingsSize => "размер настроек цветов проекта",
        Key::PaletteWarning => {
            "Некоторые цвета филаментов отсутствуют или некорректны. Эти участки показаны цветом детали или нейтральным материалом."
        }
        Key::PaintWarning => {
            "Раскраска некоторых граней не прочитана. Эти грани показаны цветом детали."
        }
        Key::FilamentEffectsWarning => "Эффекты многоцветного филамента показаны основным цветом.",
        Key::Materials => "МАТЕРИАЛЫ",
        Key::ProjectColors => "Цвета проекта",
        Key::NeutralMaterial => "Нейтральный",
        Key::Filaments => "Филаменты проекта",
        Key::NoProjectColors => "Палитры проекта нет. Используется нейтральный материал.",
        Key::UnknownColor => "Недоступен",
        Key::ColorPreviewHint => {
            "Просмотр сохранённых цветов проекта. Настройки печати и физические слоты AMS не моделируются."
        }

        Key::HelpMenu => "Справка",
        Key::Preferences => "Настройки…",
        Key::AboutApp => "О приложении YO3MFS",
        Key::Quit => "Выйти из YO3MFS",
        Key::Orientation => "Ракурс",
        Key::Projection => "Проекция",

        Key::File => "Файл",
        Key::Open => "Открыть модель",
        Key::OpenMenu => "Открыть…",
        Key::CloseWindow => "Закрыть окно",
        Key::Fit => "Показать всё",
        Key::ResetView => "Сбросить вид",
        Key::Help => "Управление",
        Key::Settings => "Настройки",
        Key::ViewTab => "Вид",
        Key::Inspector => "ИНСПЕКТОР",
        Key::SceneOverview => "Обзор сцены",
        Key::Model => "МОДЕЛЬ",
        Key::Camera => "КАМЕРА",
        Key::Navigation => "НАВИГАЦИЯ",
        Key::Appearance => "ОФОРМЛЕНИЕ",
        Key::Background => "Фон",
        Key::Language => "Язык интерфейса",
        Key::LanguageHint => "Изменения применяются сразу и сохраняются для следующего запуска.",
        Key::NoModel => "Модель не открыта",
        Key::NoModelHint => {
            "Откройте 3MF, STL, STEP, OBJ, glTF/GLB или PLY для просмотра геометрии и размеров."
        }
        Key::Dimensions => "Размеры · мм",
        Key::Instances => "Экземпляры",
        Key::Triangles => "Треугольники",
        Key::AllPlates => "Все пластины",
        Key::Plate => "Пластина {0}",
        Key::Plates => "Пластина",
        Key::Isometric => "Изометрия",
        Key::Front => "Спереди",
        Key::Back => "Сзади",
        Key::Left => "Слева",
        Key::Right => "Справа",
        Key::Top => "Сверху",
        Key::Bottom => "Снизу",
        Key::Perspective => "Перспектива",
        Key::Orthographic => "Ортографическая",
        Key::ZoomOut => "Отдалить",
        Key::ZoomIn => "Приблизить",
        Key::PanLeft => "Сдвинуть вид влево",
        Key::PanRight => "Сдвинуть вид вправо",
        Key::PanUp => "Сдвинуть вид вверх",
        Key::PanDown => "Сдвинуть вид вниз",
        Key::PanHint => "Стрелки перемещают вид",
        Key::Dark => "Графит",
        Key::Light => "Светлый",
        Key::White => "Белый",
        Key::DefaultBackground => "Сбросить фон",
        Key::ColorFormat => "Введите цвет в формате #RRGGBB.",
        Key::AutoSaved => "Сохраняется автоматически",
        Key::ImportNotes => "ОСОБЕННОСТИ ФАЙЛА",
        Key::EmptyTitle => "Каждый ракурс. Каждая деталь.",
        Key::EmptyHint => {
            "Перетащите сюда 3MF, STL, STEP, OBJ, glTF/GLB или PLY либо выберите файл"
        }
        Key::DropHint => "Отпустите, чтобы открыть 3D-модель",
        Key::Cancel => "Отмена",
        Key::Close => "Закрыть",
        Key::OtherFile => "Другой файл",
        Key::LocalReadOnly => "Локально · Только чтение",
        Key::StatusControls => "Мышь — вращение  ·  Стрелки — перемещение  ·  Колесо — масштаб",
        Key::Seconds => "{0} с",
        Key::HelpOpen => "⌘O / Ctrl+O — открыть файл",
        Key::HelpOrbit => "ЛКМ + движение — вращение",
        Key::HelpPan => "Стрелки — перемещение; Shift — быстрее",
        Key::HelpMousePan => "Средняя кнопка или Shift + ЛКМ — перемещение",
        Key::HelpKeyOrbit => "Alt + стрелки — вращение",
        Key::HelpZoom => "Колесо, pinch или + / − — масштаб",
        Key::HelpFit => "F — показать всё; Home — сбросить вид",
        Key::HelpFocus => "Нажмите на область модели для управления клавиатурой.",
        Key::HelpEscape => "Esc — закрыть диалог или отменить загрузку",
        Key::About => "О приложении",
        Key::AboutDescription => {
            "Просмотрщик 3MF, STL, STEP, OBJ, glTF/GLB и PLY. Изучайте геометрию и ракурсы, сохраняя исходные файлы неизменными."
        }
        Key::AboutIndependent => "Независимое приложение. Не связано с Bambu Lab.",
        Key::AboutVersion => "Версия {0}",
        Key::MadeInAustin => "Made in Austin ✩ Texas",
        Key::OneFile => "Откройте один файл 3MF, STL, STEP, OBJ, glTF/GLB или PLY.",
        Key::FileFilter => "Модели 3MF, STL, STEP, OBJ, glTF/GLB и PLY",
        Key::ReadArchive => "Чтение файла…",
        Key::ParseModel => "Разбор модели…",
        Key::PrepareScene => "Подготовка сцены…",
        Key::WaitLoad => "Ожидание загрузки…",
        Key::SettingsMissing => {
            "Каталог настроек недоступен. Настройки сохранятся только до закрытия."
        }
        Key::SettingsUnreadable => {
            "Настройки недоступны или слишком велики. Использованы значения по умолчанию."
        }
        Key::SettingsCorrupt => {
            "Настройки повреждены или имеют другую версию. Использованы значения по умолчанию."
        }
        Key::SaveFailed => "Изменения применены, но настройки не удалось сохранить: {0}",
        Key::GpuInit => "Не удалось инициализировать GPU.",
        Key::GpuLost => "Потеряно соединение с GPU: {0}",
        Key::GpuError => "Ошибка GPU: {0}",
        Key::GpuTitle => "yo3mfs — ошибка GPU",
        Key::GpuRestart => "{0}\nПерезапустите приложение. Исходный файл не изменён.",
        Key::GpuUpload => "Не удалось создать GPU-буферы: {0}",
        Key::GpuMemory => "Превышен лимит GPU памяти.",
        Key::GpuBuffer => "Модель превышает размер буфера этого GPU.",
        Key::GpuInstances => "Слишком много экземпляров для этого GPU.",
        Key::LaunchTitle => "yo3mfs — ошибка запуска",
        Key::LaunchError => "Не удалось создать окно или подключиться к GPU.\n{0}",
        Key::IoError => "Не удалось прочитать файл: {0}",
        Key::ZipError => "Повреждённый или неподдерживаемый ZIP: {0}",
        Key::Invalid3mf => "Некорректный 3MF: {0}",
        Key::Unsupported => "Не поддерживается обязательное расширение: {0}",
        Key::LimitExceeded => "Превышен лимит: {0}",
        Key::Cancelled => "Загрузка отменена.",
        Key::NoGeometry => {
            "Файл не содержит пригодной 3D-геометрии. Просмотр траекторий печати пока не поддерживается."
        }
        Key::RawDetail => "{0}",
        Key::VisualWarning => {
            "Материалы Core, текстуры и другие необязательные визуальные расширения не применяются."
        }
        Key::DegenerateWarning => "Пропущено вырожденных треугольников: {0}",
        Key::ExcludedWarning => {
            "Исключено служебных частей: {0}. Вычитания не применены; показана исходная геометрия."
        }
        Key::PlatesWarning => {
            "Не удалось однозначно определить пластины; показаны все объекты в исходных координатах."
        }
        Key::UnknownRole => "Неизвестная роль части: показана исходная геометрия.",
        Key::BambuWarning => "Bambu Studio: настройки печати не моделируются.",
        Key::GeometryMemory => "память геометрии",
        Key::ExpectedCore => "Ожидалась Core 3MF модель",
        Key::DuplicateResources => "Повтор resources",
        Key::DuplicateBuild => "Повтор build",
        Key::NestedObject => "Вложенный object",
        Key::MeshOutsideObject => "Mesh вне object",
        Key::MultipleMeshes => "Несколько mesh или mesh вместе с components",
        Key::VertexCount => "число вершин",
        Key::VertexMemory => "память вершин",
        Key::TriangleCount => "число треугольников",
        Key::BadIndex => "Некорректный индекс вершины",
        Key::TriangleMemory => "память треугольников",
        Key::ComponentOutside => "Component вне object",
        Key::MixedComponents => "Mesh вместе с components",
        Key::BuildCount => "число build items",
        Key::UnexpectedObject => "Неожиданный object",
        Key::IndexOutside => "Индекс треугольника вне массива вершин",
        Key::EmptyObject => "Object без mesh или components",
        Key::DuplicateObject => "Повторяющийся object ID",
        Key::GeometryOutside => "Геометрия вне mesh",
        Key::MissingResources => "Нет resources",
        Key::BadNumber => "Некорректное число",
        Key::NumberRange => "Координата или матрица вне диапазона",
        Key::MatrixSize => "Матрица 3MF должна содержать 12 чисел",
        Key::RegularFile => "Откройте один обычный файл",
        Key::RootCount => "Нужна одна корневая модель в _rels/.rels",
        Key::RootBuild => "Нет build в корневой модели",
        Key::ProductionPath => "Production path допустим только в корневой модели",
        Key::MissingRoot => "Нет корневой модели",
        Key::BuildMemory => "память build",
        Key::BoundsRange => "Габариты сцены вне доступного числового диапазона",
        Key::ComponentCount => "глубина / число компонентов",
        Key::ComponentCycle => "Циклическая ссылка компонентов",
        Key::MissingModelPart => "Нет model part",
        Key::SingularTransform => "Вырожденное преобразование",
        Key::WarningMemory => "память предупреждений",
        Key::InstanceMemory => "память экземпляров / компонентов",
        Key::InstanceCount => "число экземпляров",
        Key::RenderedTriangles => "треугольники с учётом экземпляров",
        Key::CoordinateRange => "Координаты вне числового диапазона",
        Key::BadPackagePath => "Внешний или некорректный путь в пакете",
        Key::BadUri => "Некорректный URI",
        Key::BadUriUtf8 => "Некорректный UTF-8 в URI",
        Key::ForbiddenUri => "Недопустимый URI",
        Key::PathEscape => "Путь выходит за корень пакета",
        Key::EmptyPath => "Пустой путь внутри пакета",
        Key::ArchiveSize => "размер архива",
        Key::ZipEntries => "число ZIP entries",
        Key::DuplicateZipEntry => "Неоднозначные дубликаты ZIP entries",
        Key::XmlSize => "размер XML части",
        Key::ExpandedData => "распакованные данные",
        Key::MissingRels => "Нет _rels/.rels",
        Key::BadRelsRoot => "Некорректный relationships root",
        Key::ExternalRels => "Внешние relationships запрещены",
        Key::DuplicateRel => "Повторяющийся relationship ID",
        Key::ZipEnd => "Нет корректного окончания ZIP",
        Key::MultipartZip => "Многотомный ZIP не поддерживается",
        Key::ZipCounts => "Несогласованные счётчики ZIP entries",
        Key::Zip64Locator => "Некорректный ZIP64 locator",
        Key::Zip64Directory => "Некорректный ZIP64 directory",
        Key::Zip64Counts => "Несогласованные счётчики ZIP64 entries",
        Key::ZipIndexMemory => "память ZIP directory",
        Key::PositiveId => "ID должен быть положительным",
        Key::XmlDepth => "глубина XML",
        Key::XmlRoots => "Несколько корневых XML-элементов",
        Key::XmlNameLength => "длина имени XML",
        Key::NamespaceLength => "длина namespace URI",
        Key::XmlAttributes => "размер / число XML-атрибутов",
        Key::DuplicateAttribute => "Повторяющийся атрибут",
        Key::UndeclaredExtension => "Необъявленное обязательное расширение",
        Key::XmlEnd => "Неожиданный конец XML",
        Key::ForbiddenDtd => "DTD и внешние сущности запрещены",
        Key::ForbiddenEntity => "Пользовательские XML-сущности запрещены",
        Key::XmlOutsideText => "Текст вне корня XML",
        Key::XmlIncomplete => "Незавершённый XML",
        Key::AmbiguousPlate => "Неоднозначная принадлежность Bambu plate",
        Key::SceneMemory => "CPU/GPU память сцены",
        Key::VertexBuffer => "буфер вершин",
        Key::GpuMatrixRange => "Матрица вне диапазона GPU",
        Key::UnknownUnit => "Неизвестная единица: {0}",
        Key::MissingRelationship => "Нет relationship для {0}",
        Key::MissingPart => "Не найдена часть: {0}",
        Key::MissingObject => "Не найден object {0} в {1}",
        Key::MissingAttribute => "Нет атрибута {0} у {1}",
        Key::InvalidAttribute => "Некорректный {0}",
        Key::UnknownNamespace => "Неизвестный namespace: {0}",
    }
}
