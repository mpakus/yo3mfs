use glam::DVec3;
use std::{
    io::{Cursor, Write},
    sync::atomic::AtomicBool,
};
use yo3mfs::import::{Limits, read_archive};

const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const PROD: &str = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
const REL: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const MESH: &str = r#"<mesh><vertices><vertex x="0" y="0" z="0"/><vertex x="2" y="0" z="0"/><vertex x="0" y="3" z="0"/><vertex x="0" y="0" z="4"/></vertices><triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/><triangle v1="0" v2="3" v3="2"/><triangle v1="1" v2="2" v3="3"/></triangles></mesh>"#;

fn model(resources: &str, build: &str, attrs: &str) -> String {
    format!(
        r#"<model xmlns="{CORE}" xmlns:p="{PROD}" {attrs}><resources>{resources}</resources><build>{build}</build></model>"#
    )
}
fn archive(parts: Vec<(&str, String)>) -> Cursor<Vec<u8>> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let root = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Target="/3D/custom.model" Id="root" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#
    );
    for (name, text) in [("_rels/.rels", root)].into_iter().chain(parts) {
        zip.start_file(name, options).unwrap();
        zip.write_all(text.as_bytes()).unwrap();
    }
    let mut cursor = zip.finish().unwrap();
    cursor.set_position(0);
    cursor
}
fn read(text: String) -> Result<yo3mfs::scene::Scene, yo3mfs::import::ImportError> {
    read_archive(
        archive(vec![("3D/custom.model", text)]),
        &Limits::default(),
        &AtomicBool::new(false),
    )
}
fn object() -> String {
    format!(r#"<object id="1" type="model">{MESH}</object>"#)
}

#[test]
fn discovers_root_from_relationships_and_imports_tetrahedron() {
    let s = read(model(&object(), r#"<item objectid="1"/>"#, "")).unwrap();
    assert_eq!(s.meshes.len(), 1);
    assert_eq!(s.instances.len(), 1);
    assert_eq!(s.triangle_count(None), 4);
    assert_eq!(s.bounds.min, DVec3::ZERO);
    assert_eq!(s.bounds.max, DVec3::new(2., 3., 4.));
}
#[test]
fn applies_units_nested_transforms_and_shared_instances() {
    let resources = format!(
        r#"{}<object id="2"><components><component objectid="1" transform="1 0 0 0 1 0 0 0 1 2 0 0"/></components></object>"#,
        object()
    );
    let s = read(model(
        &resources,
        r#"<item objectid="2" transform="0 1 0 -1 0 0 0 0 1 10 0 0"/><item objectid="1"/>"#,
        r#"unit="inch""#,
    ))
    .unwrap();
    assert_eq!(s.meshes.len(), 1);
    assert_eq!(s.instances.len(), 2);
    assert!((s.bounds.max - DVec3::new(254., 101.6, 101.6)).length() < 1e-8);
}
#[test]
fn supports_production_parts_with_local_ids() {
    let root = model(
        r#"<object id="1"><components><component objectid="1" p:path="/3D/Objects/part.model"/></components></object>"#,
        r#"<item objectid="1"/>"#,
        r#"requiredextensions="p""#,
    );
    let child = model(
        &object(),
        r#"<item objectid="1" transform="1 0 0 0 1 0 0 0 1 999 0 0"/>"#,
        "",
    );
    // Production external parts may omit build entirely.
    let child = child.replace(
        r#"<build><item objectid="1" transform="1 0 0 0 1 0 0 0 1 999 0 0"/></build>"#,
        "",
    );
    let rels = format!(
        r#"<Relationships xmlns="{REL}"><Relationship Target="/3D/Objects/part.model" Id="child" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>"#
    );
    let s = read_archive(
        archive(vec![
            ("3D/custom.model", root),
            ("3D/_rels/custom.model.rels", rels),
            ("3D/Objects/part.model", child),
        ]),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(s.bounds.max, DVec3::new(2., 3., 4.));
    assert_eq!(s.instances.len(), 1);
}
#[test]
fn rejects_unknown_required_extension() {
    let e = read(model(
        &object(),
        r#"<item objectid="1"/>"#,
        r#"xmlns:e="urn:unknown" requiredextensions="e""#,
    ))
    .unwrap_err();
    assert!(e.to_string().contains("urn:unknown"));
}
#[test]
fn rejects_cycles_missing_refs_and_bad_indices() {
    for (resources, build) in [
        (
            r#"<object id="1"><components><component objectid="1"/></components></object>"#
                .to_string(),
            r#"<item objectid="1"/>"#,
        ),
        (object(), r#"<item objectid="999"/>"#),
        (
            object().replace("v3=\"3\"", "v3=\"99\""),
            r#"<item objectid="1"/>"#,
        ),
    ] {
        assert!(read(model(&resources, build, "")).is_err());
    }
}
#[test]
fn rejects_nan_dtd_and_truncated_xml() {
    for text in [
        model(
            &object().replace("x=\"2\"", "x=\"NaN\""),
            r#"<item objectid="1"/>"#,
            "",
        ),
        format!(
            "<!DOCTYPE model [<!ENTITY a SYSTEM 'file:///etc/passwd'>]>{}",
            model(&object(), r#"<item objectid="1"/>"#, "")
        ),
        model(&object(), r#"<item objectid="1"/>"#, "")
            .trim_end_matches("</model>")
            .to_string(),
    ] {
        assert!(read(text).is_err());
    }
}
#[test]
fn cancellation_rejects_before_parsing() {
    assert!(read_archive(archive(vec![]), &Limits::default(), &AtomicBool::new(true)).is_err());
}
#[test]
fn warns_about_optional_visual_data() {
    let s = read(model(
        &object().replace("<mesh>", "<mesh xmlns:m=\"urn:color\" m:paint=\"red\">"),
        r#"<item objectid="1"/>"#,
        "",
    ))
    .unwrap();
    assert!(!s.warnings.is_empty());
}

#[test]
fn rejected_degenerate_geometry_does_not_expand_bounds() {
    let mesh = MESH
        .replace(
            "</vertices>",
            r#"<vertex x="1000000000" y="0" z="0"/></vertices>"#,
        )
        .replace(
            "</triangles>",
            r#"<triangle v1="4" v2="4" v3="4"/></triangles>"#,
        );
    let s = read(model(
        &format!(r#"<object id="1">{mesh}</object>"#),
        r#"<item objectid="1"/>"#,
        "",
    ))
    .unwrap();
    assert_eq!(s.bounds.max, DVec3::new(2., 3., 4.));
}
#[test]
fn bambu_instances_are_numbered_per_object() {
    let text = model(
        &format!("{}{}", object(), object().replace("id=\"1\"", "id=\"2\"")),
        r#"<item objectid="1"/><item objectid="2"/>"#,
        "",
    );
    let meta = r#"<config><plate><metadata key="plater_id" value="1"/><model_instance><metadata key="object_id" value="1"/><metadata key="instance_id" value="0"/></model_instance></plate><plate><metadata key="plater_id" value="2"/><model_instance><metadata key="object_id" value="2"/><metadata key="instance_id" value="0"/></model_instance></plate></config>"#;
    let s = read_archive(
        archive(vec![
            ("3D/custom.model", text),
            ("Metadata/model_settings.config", meta.into()),
        ]),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(
        s.instances.iter().map(|i| i.plate).collect::<Vec<_>>(),
        vec![Some(1), Some(2)]
    );
}
#[test]
fn long_names_and_xml_tokens_respect_memory_budget() {
    let limits = Limits {
        memory_bytes: 4096,
        ..Limits::default()
    };
    for text in [
        model(
            &object().replace(
                "type=\"model\"",
                &format!("name=\"{}\"", "n".repeat(10_000)),
            ),
            r#"<item objectid="1"/>"#,
            "",
        ),
        format!(
            "<!--{}-->{}",
            "x".repeat(20_000),
            model(&object(), r#"<item objectid="1"/>"#, "")
        ),
        model(
            &object(),
            r#"<item objectid="1"/>"#,
            &format!("xmlns:long=\"urn:{}\"", "x".repeat(20_000)),
        ),
    ] {
        assert!(
            read_archive(
                archive(vec![("3D/custom.model", text)]),
                &limits,
                &AtomicBool::new(false)
            )
            .is_err()
        );
    }
}

#[test]
fn unknown_roles_do_not_repeat_untrusted_strings_in_warnings() {
    let resource = object().replace(
        "type=\"model\"",
        &format!("type=\"{}\"", "unknown".repeat(200)),
    );
    let s = read(model(&resource, &r#"<item objectid="1"/>"#.repeat(200), "")).unwrap();
    assert_eq!(s.instances.len(), 200);
    assert!(
        s.warnings
            .iter()
            .map(|m| m.to_string().len())
            .sum::<usize>()
            < 512
    );
}
#[test]
fn enforces_archive_geometry_and_expansion_limits() {
    let text = model(&object(), r#"<item objectid="1"/><item objectid="1"/>"#, "");
    for limits in [
        Limits {
            archive_bytes: 10,
            ..Limits::default()
        },
        Limits {
            entries: 1,
            ..Limits::default()
        },
        Limits {
            xml_bytes: 100,
            ..Limits::default()
        },
        Limits {
            expanded_bytes: 100,
            ..Limits::default()
        },
        Limits {
            triangles: 3,
            ..Limits::default()
        },
        Limits {
            rendered_triangles: 7,
            ..Limits::default()
        },
        Limits {
            instances: 1,
            ..Limits::default()
        },
        Limits {
            memory_bytes: 1024,
            ..Limits::default()
        },
        Limits {
            xml_depth: 3,
            ..Limits::default()
        },
    ] {
        let result = read_archive(
            archive(vec![("3D/custom.model", text.clone())]),
            &limits,
            &AtomicBool::new(false),
        );
        assert!(result.is_err(), "accepted limits {limits:?}");
    }
}
#[test]
fn rejects_external_and_escaping_package_references() {
    for path in [
        "https://example.com/model",
        "../../../outside.model",
        "%2e%2e/%2e%2e/outside.model",
        "file:///outside.model",
    ] {
        let resource = format!(
            r#"<object id="1"><components><component objectid="2" p:path="{path}"/></components></object>"#
        );
        assert!(read(model(&resource, r#"<item objectid="1"/>"#, "")).is_err());
    }
}
#[test]
fn gpu_preparation_preserves_world_coordinates_and_checks_budget() {
    let text = model(
        &object(),
        r#"<item objectid="1" transform="-2 0 0 0 3 0 0 0 4 1000000000 20 30"/>"#,
        "",
    );
    let scene = read(text.clone()).unwrap();
    let center = scene.bounds.center();
    let radius = scene.bounds.radius();
    let expected = (scene.instances[0]
        .transform
        .transform_point3(scene.meshes[0].vertices[1])
        - center)
        / radius;
    let prepared =
        yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
    let matrix = glam::Mat4::from_cols_array_2d(&prepared.instances[0].matrix);
    let actual = matrix
        .transform_point3(glam::Vec3::from_array(prepared.vertices[0][1]))
        .as_dvec3();
    assert!((actual - expected).length() < 1e-6);
    assert!(
        yo3mfs::prepared::prepare(
            read(text).unwrap(),
            &Limits {
                memory_bytes: 1,
                ..Limits::default()
            },
            &AtomicBool::new(false)
        )
        .is_err()
    );
}

#[test]
fn validates_all_units_and_empty_build() {
    for (unit, scale) in [
        ("micron", 0.001),
        ("millimeter", 1.),
        ("centimeter", 10.),
        ("inch", 25.4),
        ("foot", 304.8),
        ("meter", 1000.),
    ] {
        let s = read(model(
            &object(),
            r#"<item objectid="1"/>"#,
            &format!(r#"unit="{unit}""#),
        ))
        .unwrap();
        assert!((s.bounds.size() - DVec3::new(2., 3., 4.) * scale).length() < 1e-8);
    }
    assert!(matches!(
        read(model(&object(), "", "")),
        Err(yo3mfs::import::ImportError::Empty)
    ));
}
#[test]
fn rejects_forged_large_zip_index_before_parsing() {
    let mut input = archive(vec![(
        "3D/custom.model",
        model(&object(), r#"<item objectid="1"/>"#, ""),
    )])
    .into_inner();
    let end = input.len() - 22;
    input[end + 8..end + 10].copy_from_slice(&60000u16.to_le_bytes());
    input[end + 10..end + 12].copy_from_slice(&60000u16.to_le_bytes());
    let e = read_archive(
        Cursor::new(input),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(matches!(
        e,
        yo3mfs::import::ImportError::Limit(yo3mfs::i18n::Key::ZipEntries)
    ));
}

#[test]
fn rejects_inconsistent_eocd_counts_and_supports_zip64_index() {
    let original = archive(vec![(
        "3D/custom.model",
        model(&object(), r#"<item objectid="1"/>"#, ""),
    )])
    .into_inner();
    let end = original.len() - 22;
    let mut bad = original.clone();
    bad[end + 8..end + 10].copy_from_slice(&10001u16.to_le_bytes());
    let e = read_archive(
        Cursor::new(bad),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap_err();
    assert!(e.to_string().contains("Inconsistent ZIP entry counts"));
    let size = u32::from_le_bytes(original[end + 12..end + 16].try_into().unwrap());
    let offset = u32::from_le_bytes(original[end + 16..end + 20].try_into().unwrap());
    let count = u16::from_le_bytes(original[end + 10..end + 12].try_into().unwrap()) as u64;
    let mut z = original[..end].to_vec();
    z.extend_from_slice(b"PK\x06\x06");
    z.extend_from_slice(&44u64.to_le_bytes());
    z.extend_from_slice(&45u16.to_le_bytes());
    z.extend_from_slice(&45u16.to_le_bytes());
    z.extend_from_slice(&0u32.to_le_bytes());
    z.extend_from_slice(&0u32.to_le_bytes());
    z.extend_from_slice(&count.to_le_bytes());
    z.extend_from_slice(&count.to_le_bytes());
    z.extend_from_slice(&(size as u64).to_le_bytes());
    z.extend_from_slice(&(offset as u64).to_le_bytes());
    z.extend_from_slice(b"PK\x06\x07");
    z.extend_from_slice(&0u32.to_le_bytes());
    z.extend_from_slice(&(end as u64).to_le_bytes());
    z.extend_from_slice(&1u32.to_le_bytes());
    let mut eocd = original[end..].to_vec();
    eocd[8..12].fill(0xff);
    eocd[12..20].fill(0xff);
    z.extend_from_slice(&eocd);
    assert_eq!(
        read_archive(
            Cursor::new(z.clone()),
            &Limits::default(),
            &AtomicBool::new(false)
        )
        .unwrap()
        .triangle_count(None),
        4
    );
    z[end + 24..end + 32].copy_from_slice(&10001u64.to_le_bytes());
    let e = read_archive(Cursor::new(z), &Limits::default(), &AtomicBool::new(false)).unwrap_err();
    assert!(e.to_string().contains("Inconsistent ZIP64 entry counts"));
}
