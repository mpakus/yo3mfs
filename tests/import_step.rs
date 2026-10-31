use glam::DVec3;
use std::{path::Path, sync::atomic::AtomicBool};
use yo3mfs::import::{Limits, load};

fn fixture(name: &str) -> yo3mfs::scene::Scene {
    load(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
        &Limits::default(),
        &AtomicBool::new(false),
    )
    .unwrap()
}

#[test]
fn step_box_preserves_dimensions_coordinates_and_winding() {
    let scene = fixture("sample-box.step");
    assert!(
        scene
            .bounds
            .min
            .abs_diff_eq(DVec3::new(-2., -3., -4.), 1e-5)
    );
    assert!(
        scene
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(10., 20., 30.), 1e-5)
    );
    assert_eq!(scene.triangle_count(None), 12);
    let mesh = &scene.meshes[0];
    let volume: f64 = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
            a.dot(b.cross(c)) / 6.
        })
        .sum();
    assert!((volume - 6000.).abs() < 1e-3, "{volume}");
}

#[test]
fn step_curved_surface_and_assembly_placements() {
    let cylinder = fixture("sample-cylinder.stp");
    assert!(cylinder.triangle_count(None) > 24);
    assert!(
        cylinder
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(10., 10., 20.), 0.1)
    );
    let assembly = fixture("sample-assembly.step");
    assert_eq!(assembly.triangle_count(None), 24);
    assert!(
        assembly
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(50., 20., 30.), 1e-5)
    );
}

#[test]
fn step_small_geometry_far_from_origin_survives_float_tessellation() {
    let scene = fixture("sample-distant.step");
    assert!(
        scene
            .bounds
            .min
            .abs_diff_eq(DVec3::new(999_999_998., -3., -4.), 1e-5)
    );
    assert!(
        scene
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(10., 20., 30.), 1e-5)
    );
    assert_eq!(scene.triangle_count(None), 12);
    yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
}

fn load_bytes(
    bytes: &[u8],
    limits: &Limits,
) -> Result<yo3mfs::scene::Scene, yo3mfs::import::ImportError> {
    yo3mfs::import::read_step(bytes, limits, &AtomicBool::new(false))
}

#[test]
fn step_units_are_converted_to_millimeters() {
    let text = std::fs::read_to_string("tests/fixtures/sample-box.step").unwrap();
    let meters = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
    let scene = load_bytes(meters.as_bytes(), &Limits::default()).unwrap();
    assert!(
        scene
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(10000., 20000., 30000.), 0.01)
    );
    let inches = text.replace(
        "#346 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );",
        "#346 = ( CONVERSION_BASED_UNIT('INCH',#351) LENGTH_UNIT() NAMED_UNIT(#352) );\n#351 = LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),#353);\n#352 = DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.);\n#353 = ( LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.) );"
    );
    assert_ne!(inches, text);
    let scene = load_bytes(inches.as_bytes(), &Limits::default()).unwrap();
    assert!(
        scene
            .bounds
            .min
            .abs_diff_eq(DVec3::new(-50.8, -76.2, -101.6), 0.001)
    );
    assert!(
        scene
            .bounds
            .size()
            .abs_diff_eq(DVec3::new(254., 508., 762.), 0.001)
    );
}

#[test]
fn step_errors_limits_and_cancellation_do_not_publish_partial_geometry() {
    use yo3mfs::import::{ImportError, read_step};
    let bytes = std::fs::read("tests/fixtures/sample-box.step").unwrap();
    assert!(matches!(
        read_step(bytes.as_slice(), &Limits::default(), &AtomicBool::new(true)),
        Err(ImportError::Cancelled)
    ));
    for limits in [
        Limits {
            archive_bytes: 100,
            ..Limits::default()
        },
        Limits {
            memory_bytes: 1024,
            ..Limits::default()
        },
        Limits {
            triangles: 11,
            ..Limits::default()
        },
        Limits {
            rendered_triangles: 11,
            ..Limits::default()
        },
        Limits {
            instances: 0,
            ..Limits::default()
        },
    ] {
        assert!(matches!(
            load_bytes(&bytes, &limits),
            Err(ImportError::Limit(_))
        ));
    }
    let text = String::from_utf8(bytes).unwrap();
    for invalid in [
        text[..text.len() / 2].to_owned(),
        text.replace("END-ISO-10303-21;", ""),
        text.replace("#1 = ", "#bad = "),
        text.replace(
            "ENDSEC;\nEND-ISO",
            "#999=THIS_IS_NOT_A_RECORD;\nENDSEC;\nEND-ISO",
        ),
        text.replace("DATA;", "DATA;\n#999=UNKNOWN_ENTITY('unterminated);"),
        format!("{text}garbage"),
    ] {
        assert!(
            load_bytes(invalid.as_bytes(), &Limits::default()).is_err(),
            "accepted malformed STEP"
        );
    }
    assert!(
        load_bytes(
            b"ISO-10303-21;HEADER;ENDSEC;DATA;ENDSEC;END-ISO-10303-21;",
            &Limits::default()
        )
        .is_err()
    );
}

#[test]
fn step_content_detection_unicode_source_preservation_and_localized_notes() {
    use yo3mfs::i18n::{Key, Language};
    let bytes = std::fs::read("tests/fixtures/sample-box.step").unwrap();
    let directory = tempfile::tempdir().unwrap();
    for name in ["деталь.STEP", "model.StP", "no-extension", "wrong.stl"] {
        let path = directory.path().join(name);
        std::fs::write(&path, &bytes).unwrap();
        let scene = load(&path, &Limits::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(scene.triangle_count(None), 12);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        for key in [Key::StepPreviewWarning, Key::StepConversionWarning] {
            let warning = scene.warnings.iter().find(|w| w.key == key).unwrap();
            assert_ne!(
                warning.render(Language::English),
                warning.render(Language::Russian)
            );
        }
    }
    let path = directory.path().join("ascii.step");
    std::fs::copy("tests/fixtures/sample-ascii.stl", &path).unwrap();
    assert_eq!(
        load(&path, &Limits::default(), &AtomicBool::new(false))
            .unwrap()
            .triangle_count(None),
        4
    );
}
