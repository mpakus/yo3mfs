use std::{
    io::{Cursor, Write},
    sync::atomic::AtomicBool,
};
use yo3mfs::{
    i18n::Key,
    import::{ImportError, Limits, read_archive},
    prepared::{Prepared, prepare},
    scene::{NEUTRAL, packed_color},
};

const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const PROD: &str = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const PALETTE: &str = r##"{"filament_colour":["#FF0000","#00FF00","#0000FF"]}"##;
fn mesh(triangles: &str) -> String {
    format!(
        r#"<mesh><vertices><vertex x="0" y="0" z="0"/><vertex x="4" y="0" z="0"/><vertex x="0" y="4" z="0"/></vertices><triangles>{triangles}</triangles></mesh>"#
    )
}
fn model(resources: &str, build: &str) -> String {
    format!(
        r#"<model xmlns="{CORE}" xmlns:p="{PROD}"><resources>{resources}</resources><build>{build}</build></model>"#
    )
}
fn archive(root: String, meta: &str, palette: &str, extra: Vec<(&str, String)>) -> Cursor<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let rels = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Id="root" Target="/3D/root.model" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#
    );
    for (name, text) in vec![
        ("_rels/.rels", rels),
        ("3D/root.model", root),
        ("Metadata/model_settings.config", meta.into()),
        ("Metadata/project_settings.config", palette.into()),
    ]
    .into_iter()
    .chain(extra)
    {
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    let mut bytes = zip.finish().unwrap();
    bytes.set_position(0);
    bytes
}
fn sample(
    triangles: &str,
    palette: &str,
    count: usize,
    limits: &Limits,
) -> Result<Prepared, ImportError> {
    let root = model(
        &format!(r#"<object id="1">{}</object>"#, mesh(triangles)),
        &r#"<item objectid="1"/>"#.repeat(count),
    );
    let scene = read_archive(
        archive(root, "<config/>", palette, vec![]),
        limits,
        &AtomicBool::new(false),
    )?;
    prepare(scene, limits, &AtomicBool::new(false))
}
fn face(code: &str) -> String {
    format!(r#"<triangle v1="0" v2="1" v3="2" paint_color="{code}"/>"#)
}

#[test]
fn subdivided_colors_preserve_geometry_and_exact_boundaries() {
    let p = sample(&face("481"), PALETTE, 1, &Limits::default()).unwrap();
    assert_eq!(p.scene.triangle_count(None), 1);
    assert_eq!(p.rendered_triangles, 2);
    assert_eq!(p.scene.bounds.size().to_array(), [4., 4., 0.]);
    let indices = p.indices[0].as_ref().unwrap();
    let triangles = indices
        .iter()
        .map(|t| t.map(|i| p.vertices[0][i as usize]))
        .collect::<Vec<_>>();
    assert!(
        triangles[0]
            .iter()
            .all(|v| v.color == packed_color([0, 255, 0]))
    );
    assert!(
        triangles[1]
            .iter()
            .all(|v| v.color == packed_color([255, 0, 0]))
    );
    assert_eq!(
        triangles[0].map(|v| v.position),
        [[0., 0., 0.], [-2., 2., 0.], [-2., -2., 0.]]
    );
    assert_eq!(
        triangles[1].map(|v| v.position),
        [[-2., -2., 0.], [2., -2., 0.], [0., 0., 0.]]
    );
    assert!(!p.scene.warnings.iter().any(|w| matches!(
        w.key,
        Key::PaintWarning | Key::PaletteWarning | Key::VisualWarning
    )));
}

#[test]
fn object_and_part_colors_keep_shared_external_meshes_instanced() {
    let resources = r#"<object id="1"><components><component objectid="1" p:path="/3D/part.model"/></components></object><object id="2"><components><component objectid="1" p:path="/3D/part.model"/></components></object><object id="3"><components><component objectid="1" p:path="/3D/part.model"/></components></object>"#;
    let meta = r#"<config><object id="1"><metadata key="extruder" value="2"/></object><object id="2"><metadata key="extruder" value="3"/><part id="1" subtype="normal_part"><metadata key="extruder" value="1"/></part></object><object id="3"><metadata key="extruder" value="3"/><part id="1" subtype="normal_part"><metadata key="extruder" value="0"/></part></object></config>"#;
    let rel = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Id="part" Target="/3D/part.model" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#
    );
    let part = model(
        &format!(r#"<object id="1">{}</object>"#, mesh(&face("0"))),
        "",
    );
    let scene = read_archive(
        archive(
            model(
                resources,
                r#"<item objectid="1"/><item objectid="2"/><item objectid="3"/>"#,
            ),
            meta,
            PALETTE,
            vec![("3D/_rels/root.model.rels", rel), ("3D/part.model", part)],
        ),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(scene.meshes.len(), 1);
    assert_eq!(
        scene
            .instances
            .iter()
            .map(|i| i.filament)
            .collect::<Vec<_>>(),
        [2, 1, 3]
    );
    let p = prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(
        p.instances.iter().map(|i| i.color).collect::<Vec<_>>(),
        [[0, 255, 0], [255, 0, 0], [0, 0, 255]].map(packed_color)
    );
    assert!(p.vertices[0].iter().all(|v| v.color == 0));
}

#[test]
fn whole_face_extended_slots_and_invalid_palette_keep_slot_numbers() {
    let mut colors = vec!["#FF0000"; 18];
    colors[1] = "invalid";
    colors[17] = "#123456FF";
    let palette = serde_json::json!({"filament_colour":colors}).to_string();
    let p = sample(&(face("0FC") + &face("8")), &palette, 1, &Limits::default()).unwrap();
    assert_eq!(p.scene.palette[1], None);
    assert_eq!(p.scene.palette[17], Some([18, 52, 86]));
    assert_eq!(p.rendered_triangles, 2);
    let faces = p.indices[0].as_ref().unwrap();
    assert_eq!(
        p.vertices[0][faces[0][0] as usize].color,
        packed_color([18, 52, 86])
    );
    assert_eq!(p.vertices[0][faces[1][0] as usize].color, 0);
    assert!(
        p.scene
            .warnings
            .iter()
            .any(|w| w.key == Key::PaletteWarning)
    );
}

#[test]
fn malformed_paint_falls_back_without_hiding_faces() {
    let p = sample(
        &(face("8") + &face("481x") + &face("C")),
        PALETTE,
        1,
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(p.scene.meshes[0].painting.len(), 1);
    assert_eq!(p.rendered_triangles, 3);
    assert_eq!(
        p.scene
            .warnings
            .iter()
            .filter(|w| w.key == Key::PaintWarning)
            .count(),
        1
    );
    assert_eq!(p.indices[0].as_ref().unwrap()[1..], [[0, 1, 2], [0, 1, 2]]);
}

#[test]
fn removing_degenerate_faces_keeps_the_right_paint_on_remaining_faces() {
    let triangles = face("4").replace("v2=\"1\"", "v2=\"0\"")
        + &face("8")
        + &face("0C").replace("v2=\"1\"", "v2=\"0\"")
        + &face("4");
    let p = sample(&triangles, PALETTE, 1, &Limits::default()).unwrap();
    assert_eq!(
        p.scene.meshes[0]
            .painting
            .iter()
            .map(|p| p.triangle)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(
        p.indices[0]
            .as_ref()
            .unwrap()
            .iter()
            .map(|t| p.vertices[0][t[0] as usize].color)
            .collect::<Vec<_>>(),
        [packed_color([0, 255, 0]), packed_color([255, 0, 0])]
    );
}

#[test]
fn expanded_triangle_and_memory_limits_include_paint_and_instances() {
    for (code, count, limits, key) in [
        (
            "481",
            1,
            Limits {
                rendered_triangles: 1,
                ..Limits::default()
            },
            Key::RenderedTriangles,
        ),
        (
            "481",
            2,
            Limits {
                rendered_triangles: 3,
                ..Limits::default()
            },
            Key::RenderedTriangles,
        ),
        (
            "481",
            1,
            Limits {
                paint_depth: 0,
                ..Limits::default()
            },
            Key::PaintDepth,
        ),
        (
            "4",
            1,
            Limits {
                memory_bytes: 32768,
                ..Limits::default()
            },
            Key::GeometryMemory,
        ),
    ] {
        assert!(
            matches!(sample(&face(code), PALETTE, count, &limits), Err(ImportError::Limit(k)) if k == key)
        );
    }
}

#[test]
fn corrupt_or_absent_palette_uses_neutral_without_losing_geometry() {
    for palette in ["{broken", "{}", r#"{"filament_colour":[7]}"#] {
        let p = sample(&face("4"), palette, 1, &Limits::default()).unwrap();
        assert_eq!(p.rendered_triangles, 1);
        assert_eq!(p.instances[0].color, packed_color(NEUTRAL));
        assert!(
            p.scene
                .warnings
                .iter()
                .any(|w| w.key == Key::PaletteWarning)
        );
    }
    let p = sample(
        r#"<triangle v1="0" v2="1" v3="2"/>"#,
        "{}",
        1,
        &Limits::default(),
    )
    .unwrap();
    assert!(p.indices[0].is_none());
    assert_eq!(p.vertices[0].len(), 3);
}

#[test]
fn project_json_size_and_palette_count_are_bounded() {
    let large = " ".repeat(8 * 1024 * 1024 + 1);
    assert!(matches!(
        sample(&face("4"), &large, 1, &Limits::default()),
        Err(ImportError::Limit(Key::ProjectSettingsSize))
    ));
    let palette = serde_json::json!({"filament_colour":vec!["#FFFFFF";257]}).to_string();
    let p = sample(&face("4"), &palette, 1, &Limits::default()).unwrap();
    assert!(p.scene.palette.is_empty());
    assert!(
        p.scene
            .warnings
            .iter()
            .any(|w| w.key == Key::PaletteWarning)
    );
}
