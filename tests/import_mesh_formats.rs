use glam::DVec3;
use std::{
    io::{Cursor, Read},
    sync::atomic::{AtomicBool, Ordering},
};
use yo3mfs::{
    i18n::{Key, Language},
    import::{self, ImportError, Limits},
};

const FILES: &[&str] = &[
    "sample.obj",
    "sample-ascii.ply",
    "sample-little.ply",
    "sample-big.ply",
    "sample.gltf",
    "sample.glb",
];

#[test]
fn owned_fixtures_match_manifest_world_bounds_area_and_volume() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/manifest.json")).unwrap();
    for &name in FILES {
        let scene = import::load(
            &root.join(name),
            &Limits::default(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let expected = &manifest[name];
        assert_eq!(
            scene.instances.len() as u64,
            expected["instances"].as_u64().unwrap()
        );
        assert_eq!(
            scene.triangle_count(None) as u64,
            expected["triangles"].as_u64().unwrap()
        );
        for (actual, key) in [(scene.bounds.min, "min"), (scene.bounds.max, "max")] {
            let e = &expected["bounds_mm"][key];
            let p = DVec3::new(
                e[0].as_f64().unwrap(),
                e[1].as_f64().unwrap(),
                e[2].as_f64().unwrap(),
            );
            assert!((actual - p).length() < 1e-5, "{name}: {actual} != {p}");
        }
        for instance in &scene.instances {
            let mesh = &scene.meshes[instance.mesh];
            let mut area = 0.;
            let mut volume = 0.;
            for t in &mesh.triangles {
                let [a, b, c] = t.map(|i| {
                    instance
                        .transform
                        .transform_point3(mesh.vertices[i as usize])
                });
                area += (b - a).cross(c - a).length() / 2.;
                volume += a.dot(b.cross(c)) / 6.;
            }
            assert!((area - 900.).abs() < 1e-3, "{name}: area {area}");
            assert!((volume - 1000.).abs() < 1e-3, "{name}: volume {volume}");
        }
        let bytes = scene.estimated_bytes();
        assert!(matches!(
            yo3mfs::prepared::prepare(
                scene,
                &Limits {
                    memory_bytes: bytes,
                    ..Limits::default()
                },
                &AtomicBool::new(false)
            ),
            Err(ImportError::Limit(Key::SceneMemory))
        ));
    }
}

#[test]
fn content_detection_unicode_paths_and_failed_loads_never_change_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let dir = tempfile::tempdir().unwrap();
    for &name in FILES {
        let bytes = std::fs::read(root.join(name)).unwrap();
        for suffix in [
            ".3mf",
            ".STL",
            "",
            &format!(".{}", name.rsplit('.').next().unwrap().to_uppercase()),
        ] {
            let path = dir.path().join(format!("деталь с пробелом{suffix}"));
            std::fs::write(&path, &bytes).unwrap();
            let scene = import::load(&path, &Limits::default(), &AtomicBool::new(false)).unwrap();
            assert!(!scene.meshes.is_empty());
            assert!(
                import::load(
                    &path,
                    &Limits {
                        memory_bytes: 1,
                        ..Limits::default()
                    },
                    &AtomicBool::new(false)
                )
                .is_err()
            );
            assert_eq!(std::fs::read(&path).unwrap(), bytes);
        }
    }
}

struct CancelRead<'a> {
    bytes: Cursor<Vec<u8>>,
    cancel: &'a AtomicBool,
}
impl Read for CancelRead<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let n = self.bytes.read(buffer)?;
        self.cancel.store(true, Ordering::Relaxed);
        Ok(n)
    }
}
#[test]
fn cancellation_and_actual_input_limits_cover_every_new_parser() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for &name in FILES {
        let bytes = std::fs::read(root.join(name)).unwrap();
        let cancel = AtomicBool::new(false);
        let reader = CancelRead {
            bytes: Cursor::new(bytes.clone()),
            cancel: &cancel,
        };
        let cancelled = if name.ends_with("obj") {
            import::read_obj(reader, &Limits::default(), &cancel)
        } else if name.ends_with("ply") {
            import::read_ply(reader, &Limits::default(), &cancel)
        } else {
            import::read_gltf(reader, &Limits::default(), &cancel)
        };
        assert!(
            matches!(cancelled, Err(ImportError::Cancelled)),
            "{name}: {cancelled:?}"
        );
        let limits = Limits {
            archive_bytes: bytes.len() as u64 - 1,
            ..Limits::default()
        };
        let cancel = AtomicBool::new(false);
        let result = if name.ends_with("obj") {
            import::read_obj(Cursor::new(bytes), &limits, &cancel)
        } else if name.ends_with("ply") {
            import::read_ply(Cursor::new(bytes), &limits, &cancel)
        } else {
            import::read_gltf(Cursor::new(bytes), &limits, &cancel)
        };
        assert!(matches!(result, Err(ImportError::Limit(_))), "{name}");
    }
}

#[test]
fn mesh_limits_and_localized_errors_explain_failures() {
    let huge = format!("#{}\nv 0 0 0\n", "x".repeat(65536));
    assert!(matches!(
        import::read_obj(huge.as_bytes(), &Limits::default(), &AtomicBool::new(false)),
        Err(ImportError::Limit(Key::MeshLineSize))
    ));
    let face = format!(
        "v 0 0 0\nf {}",
        std::iter::repeat_n("1", 1025).collect::<Vec<_>>().join(" ")
    );
    assert!(matches!(
        import::read_obj(face.as_bytes(), &Limits::default(), &AtomicBool::new(false)),
        Err(ImportError::Limit(Key::MeshFaceSize))
    ));
    for error in [
        ImportError::Obj(Key::MeshSyntax.into()),
        ImportError::Ply(Key::PlyPoints.into()),
        ImportError::Gltf(Key::GltfExternal.into()),
    ] {
        assert_ne!(
            error.localized(Language::English),
            error.localized(Language::Russian)
        );
    }
}
