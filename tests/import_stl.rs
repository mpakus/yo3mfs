use glam::DVec3;
use std::{io::Write, sync::atomic::AtomicBool};
use yo3mfs::import::{Limits, load};
use yo3mfs::{
    i18n::{Key, Language},
    import::ImportError,
};

const ASCII: &str = "solid triangle\nfacet normal 0 0 0\nouter loop\nvertex -2 -3 -4\nvertex 8 -3 -4\nvertex -2 17 26\nendloop\nendfacet\nendsolid triangle\n";

fn binary(header: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 80];
    bytes[..header.len()].copy_from_slice(header);
    bytes.extend(1u32.to_le_bytes());
    for value in [0f32, 0., 0., -2., -3., -4., 8., -3., -4., -2., 17., 26.] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend(0u16.to_le_bytes());
    bytes
}

fn read(bytes: &[u8]) -> Result<yo3mfs::scene::Scene, yo3mfs::import::ImportError> {
    read_with(bytes, &Limits::default(), false)
}

fn read_with(
    bytes: &[u8],
    limits: &Limits,
    cancelled: bool,
) -> Result<yo3mfs::scene::Scene, yo3mfs::import::ImportError> {
    let mut file = tempfile::Builder::new().suffix(".STL").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    load(file.path(), limits, &AtomicBool::new(cancelled))
}

#[test]
fn ascii_and_binary_preserve_coordinates_winding_and_dimensions() {
    for bytes in [
        ASCII.as_bytes().to_vec(),
        binary(b"STL test"),
        binary(b"solid binary"),
        binary(b"PK\x03\x04"),
        binary(b"PK\x05\x06"),
        binary(b"PK\x06\x06"),
    ] {
        let scene = read(&bytes).unwrap();
        assert_eq!(scene.instances.len(), 1);
        assert_eq!(scene.triangle_count(None), 1);
        assert_eq!(scene.bounds.min, DVec3::new(-2., -3., -4.));
        assert_eq!(scene.bounds.size(), DVec3::new(10., 20., 30.));
        assert_eq!(scene.meshes[0].triangles, [[0, 1, 2]]);
        assert_eq!(scene.meshes[0].vertices[1], DVec3::new(8., -3., -4.));
        let prepared =
            yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(prepared.rendered_triangles, 1);
        assert!(
            prepared.vertices[0]
                .iter()
                .all(|v| v.position.iter().all(|n| n.is_finite()))
        );
    }
}

#[test]
fn ascii_accepts_multiple_solids_whitespace_and_scientific_notation() {
    let bytes = format!(
        "\n\t{}\n{}",
        ASCII
            .replace("vertex 8", "vertex 8e0")
            .replace('\n', "\r\n"),
        ASCII
    );
    let scene = read(bytes.as_bytes()).unwrap();
    assert_eq!(scene.triangle_count(None), 2);
    assert_eq!(scene.instances.len(), 1);
    assert_eq!(scene.bounds.size(), DVec3::new(10., 20., 30.));
}

#[test]
fn content_detection_handles_wrong_extensions_unicode_paths_and_preserves_source() {
    let directory = tempfile::tempdir().unwrap();
    for (name, bytes, triangles) in [
        ("деталь с пробелом.3mf", ASCII.as_bytes(), 1),
        (
            "model.StL",
            include_bytes!("fixtures/sample.3mf").as_slice(),
            24,
        ),
        ("no-extension", binary(b"STL").as_slice(), 1),
    ] {
        let path = directory.path().join(name);
        std::fs::write(&path, bytes).unwrap();
        let scene = load(&path, &Limits::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(scene.triangle_count(None), triangles);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn invalid_binary_counts_truncation_and_trailing_data_are_rejected() {
    let bytes = binary(b"solid binary");
    for end in 0..bytes.len() {
        assert!(read(&bytes[..end]).is_err(), "accepted truncation at {end}");
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(read(&extra).is_err());
    for count in [0u32, 2, u32::MAX] {
        let mut forged = bytes.clone();
        forged[80..84].copy_from_slice(&count.to_le_bytes());
        assert!(read(&forged).is_err());
    }
}

#[test]
fn malformed_ascii_cannot_publish_partial_geometry() {
    for text in [
        ASCII.replace("endsolid triangle", ""),
        ASCII.replace("endfacet", ""),
        ASCII.replace("outer loop", "outer ring"),
        ASCII.replace("vertex 8 -3 -4", ""),
        ASCII.replace("endloop", "vertex 0 0 0\nendloop"),
        format!("{ASCII}garbage\n"),
        format!("{ASCII}solid incomplete\nfacet normal 0 0 1\n"),
    ] {
        assert!(
            matches!(read(text.as_bytes()), Err(ImportError::Stl(_))),
            "{text}"
        );
    }
}

#[test]
fn nonfinite_and_out_of_range_numbers_are_rejected_in_both_encodings() {
    for value in ["NaN", "inf", "-inf", "1e999", "1000000000000001"] {
        let text = ASCII.replace("vertex 8", &format!("vertex {value}"));
        assert!(matches!(read(text.as_bytes()), Err(ImportError::Stl(_))));
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e16] {
        let mut bytes = binary(b"STL");
        bytes[96..100].copy_from_slice(&value.to_le_bytes());
        assert!(matches!(read(&bytes), Err(ImportError::Stl(_))));
    }
    let text = ASCII.replace("normal 0 0 0", "normal NaN 0 0");
    assert!(read(text.as_bytes()).is_err());
    let mut bytes = binary(b"STL");
    bytes[84..88].copy_from_slice(&f32::NAN.to_le_bytes());
    assert!(read(&bytes).is_err());
}

#[test]
fn degenerate_faces_warn_and_do_not_inflate_bounds() {
    let degenerate = "solid point\nfacet normal 0 0 0\nouter loop\nvertex 100 100 100\nvertex 100 100 100\nvertex 100 100 100\nendloop\nendfacet\nendsolid point\n";
    let scene = read(format!("{ASCII}{degenerate}").as_bytes()).unwrap();
    assert_eq!(scene.triangle_count(None), 1);
    assert_eq!(scene.bounds.size(), DVec3::new(10., 20., 30.));
    assert_eq!(scene.meshes[0].vertices.len(), 3);
    let warning = scene
        .warnings
        .iter()
        .find(|w| w.key == Key::DegenerateWarning)
        .unwrap();
    assert_eq!(
        warning.render(Language::English),
        "Degenerate triangles skipped: 1"
    );
    assert!(matches!(
        read(degenerate.as_bytes()),
        Err(ImportError::Empty)
    ));
    assert!(matches!(
        read(b"solid empty\nendsolid empty\n"),
        Err(ImportError::Empty)
    ));
    assert!(matches!(read(&[0; 84]), Err(ImportError::Empty)));
    let limits = Limits {
        triangles: 1,
        ..Limits::default()
    };
    assert!(matches!(
        read_with(
            format!("{degenerate}{degenerate}").as_bytes(),
            &limits,
            false
        ),
        Err(ImportError::Limit(Key::TriangleCount))
    ));
}

#[test]
fn units_and_unsupported_color_warnings_are_persistent_and_localized() {
    for header in [b"STL".as_slice(), b"COLOR=abcd", b"MATERIAL=abcd"] {
        let mut bytes = binary(header);
        bytes[132..134].copy_from_slice(&0x801fu16.to_le_bytes());
        let scene = read(&bytes).unwrap();
        assert!(scene.palette.is_empty());
        assert!(scene.meshes[0].painting.is_empty());
        for key in [Key::StlUnitsWarning, Key::StlAttributesWarning] {
            let warning = scene.warnings.iter().find(|w| w.key == key).unwrap();
            assert_ne!(
                warning.render(Language::English),
                warning.render(Language::Russian)
            );
        }
    }
    let scene = read(&binary(b"COLOR=abcd")).unwrap();
    assert!(
        scene
            .warnings
            .iter()
            .any(|w| w.key == Key::StlAttributesWarning)
    );
    let scene = read(ASCII.as_bytes()).unwrap();
    assert_eq!(scene.warnings, vec![Key::StlUnitsWarning.into()]);
    let error = read(ASCII.replace("outer loop", "wrong").as_bytes()).unwrap_err();
    assert!(
        error
            .localized(Language::English)
            .starts_with("Invalid STL:")
    );
    assert!(
        error
            .localized(Language::Russian)
            .starts_with("Некорректный STL:")
    );
}

#[test]
fn input_geometry_instance_and_rendering_limits_are_enforced() {
    for bytes in [ASCII.as_bytes().to_vec(), binary(b"STL")] {
        for (limits, key) in [
            (
                Limits {
                    archive_bytes: bytes.len() as u64 - 1,
                    ..Limits::default()
                },
                Key::InputSize,
            ),
            (
                Limits {
                    memory_bytes: 1,
                    ..Limits::default()
                },
                Key::GeometryMemory,
            ),
            (
                Limits {
                    triangles: 0,
                    ..Limits::default()
                },
                Key::TriangleCount,
            ),
            (
                Limits {
                    rendered_triangles: 0,
                    ..Limits::default()
                },
                Key::RenderedTriangles,
            ),
            (
                Limits {
                    instances: 0,
                    ..Limits::default()
                },
                Key::InstanceCount,
            ),
        ] {
            assert!(
                matches!(read_with(&bytes, &limits, false), Err(ImportError::Limit(actual)) if actual == key)
            );
        }
        assert!(matches!(
            read_with(&bytes, &Limits::default(), true),
            Err(ImportError::Cancelled)
        ));
        let limits = Limits {
            archive_bytes: bytes.len() as u64,
            triangles: 1,
            rendered_triangles: 1,
            instances: 1,
            ..Limits::default()
        };
        assert!(read_with(&bytes, &limits, false).is_ok());
        let scene = read(&bytes).unwrap();
        let limits = Limits {
            memory_bytes: scene.estimated_bytes(),
            ..Limits::default()
        };
        assert!(matches!(
            yo3mfs::prepared::prepare(scene, &limits, &AtomicBool::new(false)),
            Err(ImportError::Limit(Key::SceneMemory))
        ));
    }
    let huge_line = format!("solid {}\nendsolid\n", "x".repeat(65536));
    assert!(matches!(
        read(huge_line.as_bytes()),
        Err(ImportError::Limit(Key::StlLineSize))
    ));
}

#[test]
fn cancellation_is_checked_during_streaming() {
    use std::io::{Cursor, Read, Seek, SeekFrom};
    use std::sync::atomic::Ordering;
    struct CancelReader<'a> {
        input: Cursor<Vec<u8>>,
        cancel: &'a AtomicBool,
        read: usize,
    }
    impl Read for CancelReader<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.input.read(buf)?;
            self.read += n;
            if self.read > 84 {
                self.cancel.store(true, Ordering::Relaxed);
            }
            Ok(n)
        }
    }
    impl Seek for CancelReader<'_> {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            self.input.seek(pos)
        }
    }
    for bytes in [ASCII.repeat(1000).into_bytes(), binary(b"STL")] {
        let cancel = AtomicBool::new(false);
        let reader = CancelReader {
            input: Cursor::new(bytes),
            cancel: &cancel,
            read: 0,
        };
        assert!(matches!(
            yo3mfs::import::read_stl(reader, &Limits::default(), &cancel),
            Err(ImportError::Cancelled)
        ));
    }
}

#[test]
fn streaming_checks_actual_bytes_when_input_grows_after_size_check() {
    use std::io::{Cursor, Read, Seek, SeekFrom};
    struct GrowingReader(Cursor<Vec<u8>>);
    impl Read for GrowingReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Seek for GrowingReader {
        fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
            if pos == SeekFrom::End(0) {
                Ok(1)
            } else {
                self.0.seek(pos)
            }
        }
    }
    let limits = Limits {
        archive_bytes: ASCII.len() as u64,
        ..Limits::default()
    };
    let input = GrowingReader(Cursor::new(ASCII.repeat(2).into_bytes()));
    assert!(matches!(
        yo3mfs::import::read_stl(input, &limits, &AtomicBool::new(false)),
        Err(ImportError::Limit(Key::InputSize))
    ));
}

#[test]
fn checked_in_tetrahedra_match_provenance_manifest_and_surface_area() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/manifest.json")).unwrap();
    for name in ["sample-ascii.stl", "sample-binary.stl"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let scene = load(&path, &Limits::default(), &AtomicBool::new(false)).unwrap();
        assert_eq!(
            scene.triangle_count(None) as u64,
            manifest[name]["triangles"].as_u64().unwrap()
        );
        assert_eq!(scene.instances.len(), 1);
        assert_eq!(scene.bounds.min, DVec3::new(-2., -3., -4.));
        assert_eq!(scene.bounds.max, DVec3::new(8., 17., 26.));
        let mesh = &scene.meshes[0];
        let area: f64 = mesh
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                (b - a).cross(c - a).length() * 0.5
            })
            .sum();
        assert!((area - 900.).abs() < 1e-10);
        let volume: f64 = mesh
            .triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                a.dot(b.cross(c)) / 6.
            })
            .sum();
        assert!((volume - 1000.).abs() < 1e-10);
    }
}
