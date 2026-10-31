use glam::DVec3;
use std::{io::Write, sync::atomic::AtomicBool};
use yo3mfs::{
    import::{ImportError, Limits, load},
    scene::Scene,
};

fn read(text: &str, limits: &Limits) -> Result<Scene, ImportError> {
    let mut file = tempfile::Builder::new().suffix(".OBJ").tempfile().unwrap();
    file.write_all(text.as_bytes()).unwrap();
    load(file.path(), limits, &AtomicBool::new(false))
}

#[test]
fn obj_preserves_negative_coordinates_and_relative_face_indices() {
    let scene = read(
        "# test\nv -2 -3 -4\nv 8 -3 -4\nv -2 17 26\nf -3 -2 -1\n",
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(scene.bounds.min, DVec3::new(-2., -3., -4.));
    assert_eq!(scene.bounds.size(), DVec3::new(10., 20., 30.));
    assert_eq!(scene.meshes[0].triangles, [[0, 1, 2]]);
}

#[test]
fn obj_triangulates_concave_faces_without_changing_area_or_winding() {
    let scene = read(
        "v 0 0 0\nv 2 0 0\nv 2 2 0\nv 1 1 0\nv 0 2 0\nf 1 2 3 4 5\n",
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(scene.triangle_count(None), 3);
    let mesh = &scene.meshes[0];
    let area: f64 = mesh
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
            let z = (b - a).cross(c - a).z;
            assert!(z > 0.);
            z / 2.
        })
        .sum();
    assert!((area - 3.).abs() < 1e-12);
}

#[test]
fn obj_handles_groups_slash_indices_and_material_notice() {
    let scene = read("mtllib ../../secret.mtl\no first\nv 0 0 0\nv 1 0 0\nv 0 1 0\nvt 0 0\nvn 0 0 1\nf 1/1/1 2/1/1 3/1/1\ng second\nf 1//1 3//1 2//1\n", &Limits::default()).unwrap();
    assert_eq!(scene.triangle_count(None), 2);
    assert!(scene.warnings.len() >= 2);
}

#[test]
fn obj_rejects_bad_geometry_and_exhausted_budgets() {
    for text in [
        "v NaN 0 0\nf 1 1 1",
        "v 0 0 0\nf 0 1 2",
        "v 0 0 0\nf 1 2 3",
        "surf 0 1 0 1 1 2 3",
        "v 0 0 0\nv 1 1 0\nv 0 1 0\nv 1 0 0\nf 1 2 3 4",
    ] {
        assert!(read(text, &Limits::default()).is_err(), "{text}");
    }
    let text = "v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";
    for limits in [
        Limits {
            triangles: 0,
            ..Limits::default()
        },
        Limits {
            memory_bytes: 1,
            ..Limits::default()
        },
        Limits {
            archive_bytes: 10,
            ..Limits::default()
        },
        Limits {
            instances: 0,
            ..Limits::default()
        },
    ] {
        assert!(matches!(read(text, &limits), Err(ImportError::Limit(_))));
    }
}

#[test]
fn unused_vertices_do_not_expand_displayed_surface_bounds() {
    let scene = read(
        "v 0 0 0\nv 1 0 0\nv 0 1 0\nv 999 999 999\nf 1 2 3\n",
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(scene.bounds.size(), DVec3::new(1., 1., 0.));
}
