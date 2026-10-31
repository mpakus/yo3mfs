use super::Key;

pub(super) const fn get(key: Key) -> &'static str {
    match key {
        Key::EmbeddedColors => {
            "Embedded mesh colors are displayed. This model has no filament palette."
        }
        Key::InvalidModel => "Invalid model: {0}",
        Key::InvalidObj => "Invalid OBJ: {0}",
        Key::InvalidPly => "Invalid PLY: {0}",
        Key::InvalidGltf => "Invalid glTF/GLB: {0}",
        Key::MeshSyntax => "Malformed or incomplete mesh data.",
        Key::MeshPolygon => {
            "A face is degenerate, nonplanar, self-intersecting, or cannot be triangulated."
        }
        Key::MeshLineSize => "mesh text line length (64 KiB maximum)",
        Key::MeshFaceSize => "polygon vertex count (1,024 maximum)",
        Key::MeshUnitsWarning => {
            "This format has no reliable unit metadata. Coordinates are interpreted as millimeters."
        }
        Key::ObjMaterialsWarning => {
            "OBJ materials and textures are not loaded; external MTL files are never opened. Embedded vertex colors are displayed when present."
        }
        Key::MeshAttributesWarning => {
            "Additional attributes, stored normals, or non-surface data are not rendered. Shading uses face geometry."
        }
        Key::MeshUnsupported => {
            "This geometry feature is not supported by the static mesh preview. Export a polygon mesh instead."
        }
        Key::PlyPoints => {
            "PLY contains points without faces. Export a surface mesh to view this file."
        }
        Key::GltfExternal => {
            "External glTF buffers are not opened. Export a self-contained GLB or glTF with embedded buffers."
        }
        Key::GltfPreviewWarning => {
            "Static glTF preview: base colors are shown with simple lighting. Textures, transparency, PBR effects, cameras, and animation are not reproduced."
        }
        Key::GltfSceneWarning => {
            "Only the default scene (or the first scene when no default is set) is displayed."
        }
        Key::GltfDeformation => {
            "Skins and morph targets are not supported. Export a static mesh with deformation applied."
        }
        Key::GltfInputSize => {
            "glTF input or decoded buffer size (64 MiB maximum; JSON 16 MiB maximum)"
        }
        Key::GltfStructure => "Invalid glTF scene, buffer, accessor, or transform.",
        Key::GltfExtensionsWarning => {
            "Optional glTF extensions are not applied; the core mesh preview may differ."
        }

        Key::StepFaces => "STEP face count (100,000 maximum)",
        Key::StepKernelSize => "The replacement STEP kernel exceeds 32 MiB.",
        Key::StepKernelTrap => "The STEP CAD engine stopped unexpectedly. Try another model.",
        Key::InvalidStep => "Invalid STEP: {0}",
        Key::StepFormat => {
            "Expected a complete STEP Part 21 file (.step or .stp). Compressed STEP and external model files are not supported."
        }
        Key::StepInputSize => "STEP input size (64 MiB maximum)",
        Key::StepMemory => "STEP conversion memory",
        Key::StepTimeout => "STEP conversion exceeded its 60-second time limit.",
        Key::StepKernel => "STEP CAD engine failed: {0}",
        Key::StepIncomplete => "STEP contains missing, ambiguous, invalid, or untessellated faces.",
        Key::StepPreviewWarning => {
            "STEP preview: curved surfaces are tessellated in millimeters and assemblies are combined. Colors, names, PMI, and external files are not loaded."
        }
        Key::StepConversionWarning => {
            "The CAD engine may normalize or heal STEP topology and omit unsupported entities. Complete root transfer cannot be verified; compare this preview with the source CAD model."
        }
        Key::InvalidStl => "Invalid STL: {0}",
        Key::StlFormat => {
            "Expected ASCII STL or binary STL with a matching facet count. Open a 3MF, STL, STEP, OBJ, glTF/GLB, or PLY model."
        }
        Key::StlLength => "Binary STL is truncated or has data beyond its declared facet count.",
        Key::StlSyntax => "Malformed ASCII STL near line {0}.",
        Key::StlNumber => "STL normals and coordinates must be finite numbers.",
        Key::StlLineSize => "STL line length",
        Key::InputSize => "input file size",
        Key::StlUnitsWarning => "STL has no units. Coordinates are interpreted as millimeters.",
        Key::StlAttributesWarning => {
            "STL color or attribute data is not applied. Geometry is shown in a neutral material."
        }
        Key::CliHelp => {
            "yo3mfs {0}\nUsage: yo3mfs [path/to/model.3mf | path/to/model.stl | path/to/model.step | path/to/model.obj | path/to/model.glb | path/to/model.gltf | path/to/model.ply]\nOffline 3MF, STL, STEP, OBJ, glTF/GLB, and PLY viewer. Input files are never modified."
        }
        Key::BadPaint => "Invalid Bambu facet paint",
        Key::PaintDepth => "paint subdivision depth",
        Key::ProjectSettingsSize => "project color settings size",
        Key::PaletteWarning => {
            "Some filament colors are missing or invalid. Those areas use the part color or neutral material."
        }
        Key::PaintWarning => "Some facet paint could not be read. Those faces use the part color.",
        Key::FilamentEffectsWarning => {
            "Multicolor filament effects are shown using the base color."
        }
        Key::Materials => "MATERIALS",
        Key::ProjectColors => "Project colors",
        Key::NeutralMaterial => "Neutral",
        Key::Filaments => "Project filaments",
        Key::NoProjectColors => "No project palette. Neutral material is used.",
        Key::UnknownColor => "Unavailable",
        Key::ColorPreviewHint => {
            "Preview of saved project colors. Printing settings and physical AMS slots are not simulated."
        }

        Key::HelpMenu => "Help",
        Key::Preferences => "Settings…",
        Key::AboutApp => "About YO3MFS",
        Key::Quit => "Quit YO3MFS",
        Key::Orientation => "Orientation",
        Key::Projection => "Projection",

        Key::File => "File",
        Key::Open => "Open model",
        Key::OpenMenu => "Open…",
        Key::CloseWindow => "Close window",
        Key::Fit => "Fit all",
        Key::ResetView => "Reset view",
        Key::Help => "Controls",
        Key::Settings => "Settings",
        Key::ViewTab => "View",
        Key::Inspector => "INSPECTOR",
        Key::SceneOverview => "Scene overview",
        Key::Model => "MODEL",
        Key::Camera => "CAMERA",
        Key::Navigation => "NAVIGATION",
        Key::Appearance => "APPEARANCE",
        Key::Background => "Background",
        Key::Language => "Interface language",
        Key::LanguageHint => "Changes apply immediately and are saved for next time.",
        Key::NoModel => "No model open",
        Key::NoModelHint => {
            "Open a 3MF, STL, STEP, OBJ, glTF/GLB, or PLY to inspect its geometry and dimensions."
        }
        Key::Dimensions => "Dimensions · mm",
        Key::Instances => "Instances",
        Key::Triangles => "Triangles",
        Key::AllPlates => "All plates",
        Key::Plate => "Plate {0}",
        Key::Plates => "Build plate",
        Key::Isometric => "Isometric",
        Key::Front => "Front",
        Key::Back => "Back",
        Key::Left => "Left",
        Key::Right => "Right",
        Key::Top => "Top",
        Key::Bottom => "Bottom",
        Key::Perspective => "Perspective",
        Key::Orthographic => "Orthographic",
        Key::ZoomOut => "Zoom out",
        Key::ZoomIn => "Zoom in",
        Key::PanLeft => "Move view left",
        Key::PanRight => "Move view right",
        Key::PanUp => "Move view up",
        Key::PanDown => "Move view down",
        Key::PanHint => "Arrow keys move the view",
        Key::Dark => "Graphite",
        Key::Light => "Mist",
        Key::White => "White",
        Key::DefaultBackground => "Reset background",
        Key::ColorFormat => "Use a color in #RRGGBB format.",
        Key::AutoSaved => "Saved automatically",
        Key::ImportNotes => "IMPORT NOTES",
        Key::EmptyTitle => "Every angle. Every detail.",
        Key::EmptyHint => "Drop a 3MF, STL, STEP, OBJ, glTF/GLB, or PLY here or choose a file",
        Key::DropHint => "Release to open a 3D model",
        Key::Cancel => "Cancel",
        Key::Close => "Dismiss",
        Key::OtherFile => "Choose another file",
        Key::LocalReadOnly => "Offline · Read only",
        Key::StatusControls => "Drag to orbit  ·  Arrows to pan  ·  Scroll to zoom",
        Key::Seconds => "{0} s",
        Key::HelpOpen => "⌘O / Ctrl+O — open a file",
        Key::HelpOrbit => "Drag with the left mouse button to orbit",
        Key::HelpPan => "Arrow keys — move view; Shift — move faster",
        Key::HelpMousePan => "Middle button or Shift + drag — pan",
        Key::HelpKeyOrbit => "Alt + arrow keys — orbit",
        Key::HelpZoom => "Scroll, pinch or + / − — zoom",
        Key::HelpFit => "F — fit all; Home — reset view",
        Key::HelpFocus => "Click the viewport to use keyboard controls.",
        Key::HelpEscape => "Esc — close a dialog or cancel loading",
        Key::About => "About",
        Key::AboutDescription => {
            "A focused 3MF, STL, STEP, OBJ, glTF/GLB, and PLY viewer. Inspect geometry, explore every angle, and keep your files untouched."
        }
        Key::AboutIndependent => "Independent software. Not affiliated with Bambu Lab.",
        Key::AboutVersion => "Version {0}",
        Key::MadeInAustin => "Made in Austin ✩ Texas",
        Key::OneFile => "Open one 3MF, STL, STEP, OBJ, glTF/GLB, or PLY file at a time.",
        Key::FileFilter => "3MF, STL, STEP, OBJ, glTF/GLB, and PLY models",
        Key::ReadArchive => "Reading file…",
        Key::ParseModel => "Parsing model…",
        Key::PrepareScene => "Preparing scene…",
        Key::WaitLoad => "Waiting to load…",
        Key::SettingsMissing => {
            "Settings folder unavailable. Preferences will last for this session only."
        }
        Key::SettingsUnreadable => "Settings are unavailable or too large. Defaults restored.",
        Key::SettingsCorrupt => "Settings are damaged or use another version. Defaults restored.",
        Key::SaveFailed => "Changes applied, but preferences could not be saved: {0}",
        Key::GpuInit => "Could not initialize the GPU.",
        Key::GpuLost => "Lost connection to the GPU: {0}",
        Key::GpuError => "GPU error: {0}",
        Key::GpuTitle => "yo3mfs — GPU error",
        Key::GpuRestart => "{0}\nRestart the app. Your original file has not been changed.",
        Key::GpuUpload => "Could not create GPU buffers: {0}",
        Key::GpuMemory => "GPU memory limit exceeded.",
        Key::GpuBuffer => "The model exceeds this GPU's buffer size limit.",
        Key::GpuInstances => "Too many instances for this GPU.",
        Key::LaunchTitle => "yo3mfs — launch error",
        Key::LaunchError => "Could not create the window or connect to the GPU.\n{0}",
        Key::IoError => "Could not read the file: {0}",
        Key::ZipError => "Damaged or unsupported ZIP: {0}",
        Key::Invalid3mf => "Invalid 3MF: {0}",
        Key::Unsupported => "Unsupported required extension: {0}",
        Key::LimitExceeded => "Limit exceeded: {0}",
        Key::Cancelled => "Loading cancelled.",
        Key::NoGeometry => {
            "This file contains no usable 3D geometry. Toolpath preview is not supported."
        }
        Key::RawDetail => "{0}",
        Key::VisualWarning => {
            "Core materials, textures and other optional visual extensions are not applied."
        }
        Key::DegenerateWarning => "Degenerate triangles skipped: {0}",
        Key::ExcludedWarning => {
            "Helper parts excluded: {0}. Subtractions are not applied; original geometry is shown."
        }
        Key::PlatesWarning => {
            "Plates could not be resolved unambiguously. All objects are shown in their original coordinates."
        }
        Key::UnknownRole => "Unknown part role: original geometry is shown.",
        Key::BambuWarning => "Bambu Studio: print settings are not simulated.",
        Key::GeometryMemory => "geometry memory",
        Key::ExpectedCore => "Expected a Core 3MF model",
        Key::DuplicateResources => "Duplicate resources section",
        Key::DuplicateBuild => "Duplicate build section",
        Key::NestedObject => "Nested object element",
        Key::MeshOutsideObject => "Mesh outside object",
        Key::MultipleMeshes => "Multiple meshes or mesh mixed with components",
        Key::VertexCount => "vertex count",
        Key::VertexMemory => "vertex memory",
        Key::TriangleCount => "triangle count",
        Key::BadIndex => "Invalid vertex index",
        Key::TriangleMemory => "triangle memory",
        Key::ComponentOutside => "Component outside object",
        Key::MixedComponents => "Mesh mixed with components",
        Key::BuildCount => "build item count",
        Key::UnexpectedObject => "Unexpected object element",
        Key::IndexOutside => "Triangle index outside vertex array",
        Key::EmptyObject => "Object without mesh or components",
        Key::DuplicateObject => "Duplicate object ID",
        Key::GeometryOutside => "Geometry outside mesh",
        Key::MissingResources => "Missing resources section",
        Key::BadNumber => "Invalid number",
        Key::NumberRange => "Coordinate or matrix outside numeric range",
        Key::MatrixSize => "A 3MF transform must contain 12 numbers",
        Key::RegularFile => "Open a single regular file",
        Key::RootCount => "Exactly one root model is required in _rels/.rels",
        Key::RootBuild => "Root model has no build section",
        Key::ProductionPath => "Production path is only allowed in the root model",
        Key::MissingRoot => "Missing root model",
        Key::BuildMemory => "build memory",
        Key::BoundsRange => "Scene bounds outside numeric range",
        Key::ComponentCount => "component depth / count",
        Key::ComponentCycle => "Cyclic component reference",
        Key::MissingModelPart => "Missing model part",
        Key::SingularTransform => "Singular transform",
        Key::WarningMemory => "warning memory",
        Key::InstanceMemory => "instance / component memory",
        Key::InstanceCount => "instance count",
        Key::RenderedTriangles => "triangle count including instances",
        Key::CoordinateRange => "Coordinates outside numeric range",
        Key::BadPackagePath => "External or invalid package path",
        Key::BadUri => "Invalid URI",
        Key::BadUriUtf8 => "Invalid UTF-8 in URI",
        Key::ForbiddenUri => "Disallowed URI",
        Key::PathEscape => "Path escapes the package root",
        Key::EmptyPath => "Empty package path",
        Key::ArchiveSize => "archive size",
        Key::ZipEntries => "ZIP entry count",
        Key::DuplicateZipEntry => "Ambiguous duplicate ZIP entries",
        Key::XmlSize => "XML part size",
        Key::ExpandedData => "expanded data",
        Key::MissingRels => "Missing _rels/.rels",
        Key::BadRelsRoot => "Invalid relationships root",
        Key::ExternalRels => "External relationships are forbidden",
        Key::DuplicateRel => "Duplicate relationship ID",
        Key::ZipEnd => "Missing valid ZIP ending",
        Key::MultipartZip => "Multipart ZIP is not supported",
        Key::ZipCounts => "Inconsistent ZIP entry counts",
        Key::Zip64Locator => "Invalid ZIP64 locator",
        Key::Zip64Directory => "Invalid ZIP64 directory",
        Key::Zip64Counts => "Inconsistent ZIP64 entry counts",
        Key::ZipIndexMemory => "ZIP directory memory",
        Key::PositiveId => "ID must be positive",
        Key::XmlDepth => "XML depth",
        Key::XmlRoots => "Multiple XML roots",
        Key::XmlNameLength => "XML name length",
        Key::NamespaceLength => "namespace URI length",
        Key::XmlAttributes => "XML attribute size / count",
        Key::DuplicateAttribute => "Duplicate XML attribute",
        Key::UndeclaredExtension => "Undeclared required extension",
        Key::XmlEnd => "Unexpected XML end",
        Key::ForbiddenDtd => "DTD and external entities are forbidden",
        Key::ForbiddenEntity => "Custom XML entities are forbidden",
        Key::XmlOutsideText => "Text outside the XML root",
        Key::XmlIncomplete => "Incomplete XML",
        Key::AmbiguousPlate => "Ambiguous Bambu plate assignment",
        Key::SceneMemory => "CPU/GPU scene memory",
        Key::VertexBuffer => "vertex buffer",
        Key::GpuMatrixRange => "Matrix outside GPU numeric range",
        Key::UnknownUnit => "Unknown unit: {0}",
        Key::MissingRelationship => "Missing relationship for {0}",
        Key::MissingPart => "Missing part: {0}",
        Key::MissingObject => "Object {0} not found in {1}",
        Key::MissingAttribute => "Missing attribute {0} on {1}",
        Key::InvalidAttribute => "Invalid {0}",
        Key::UnknownNamespace => "Unknown namespace: {0}",
    }
}
