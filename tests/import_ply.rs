use glam::DVec3;
use std::{io::Write, sync::atomic::AtomicBool};
use yo3mfs::{
    import::{Limits, load},
    scene::packed_color,
};

fn read(bytes: &[u8]) -> Result<yo3mfs::scene::Scene, yo3mfs::import::ImportError> {
    let mut file = tempfile::Builder::new().suffix(".ply").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    load(file.path(), &Limits::default(), &AtomicBool::new(false))
}
fn header(format: &str) -> String {
    format!(
        "ply\nformat {format} 1.0\ncomment generated test\nelement vertex 3\nproperty float x\nproperty float y\nproperty float z\nproperty uchar red\nproperty uchar green\nproperty uchar blue\nelement face 1\nproperty list uchar int vertex_indices\nend_header\n"
    )
}
#[test]
fn ascii_and_both_binary_orders_preserve_geometry_and_colors() {
    let points = [[-2f32, -3., -4.], [8., -3., -4.], [-2., 17., 26.]];
    for format in ["ascii", "binary_little_endian", "binary_big_endian"] {
        let mut bytes = header(format).into_bytes();
        if format == "ascii" {
            bytes.extend(b"-2 -3 -4 255 0 0\n8 -3 -4 0 255 0\n-2 17 26 0 0 255\n3 0 1 2\n");
        } else {
            for (i, p) in points.iter().enumerate() {
                for x in p {
                    bytes.extend(if format == "binary_little_endian" {
                        x.to_le_bytes()
                    } else {
                        x.to_be_bytes()
                    });
                }
                let mut rgb = [0; 3];
                rgb[i] = 255;
                bytes.extend(rgb);
            }
            bytes.push(3);
            for i in 0i32..3 {
                bytes.extend(if format == "binary_little_endian" {
                    i.to_le_bytes()
                } else {
                    i.to_be_bytes()
                });
            }
        }
        let scene = read(&bytes).unwrap();
        assert_eq!(scene.bounds.size(), DVec3::new(10., 20., 30.));
        assert_eq!(scene.meshes[0].triangles, [[0, 1, 2]]);
        assert_eq!(
            scene.meshes[0].colors,
            [[255, 0, 0], [0, 255, 0], [0, 0, 255]].map(packed_color)
        );
    }
}
#[test]
fn truncated_invalid_indices_and_point_clouds_are_rejected() {
    let good = format!(
        "{}0 0 0 255 0 0\n1 0 0 0 255 0\n0 1 0 0 0 255\n3 0 1 2\n",
        header("ascii")
    );
    for text in [
        good.replace("3 0 1 2", "3 0 1 9"),
        good.replace("3 0 1 2", "-1"),
        good.replace("0 0 0 255", "NaN 0 0 255"),
        header("ascii"),
    ] {
        assert!(read(text.as_bytes()).is_err());
    }
    assert!(read(b"ply\nformat ascii 1.0\nelement vertex 1\nproperty float x\nproperty float y\nproperty float z\nend_header\n0 0 0\n").is_err());
}

#[test]
fn reordered_properties_unknown_elements_and_concave_faces_remain_aligned() {
    let text = "ply\nformat ascii 1.0\nelement vertex 5\nproperty double z\nproperty double x\nproperty float confidence\nproperty double y\nelement edge 1\nproperty int a\nproperty int b\nelement face 1\nproperty list uchar int vertex_index\nend_header\n0 0 0.5 0\n0 2 0.5 0\n0 2 0.5 2\n0 1 0.5 1\n0 0 0.5 2\n0 1\n5 0 1 2 3 4\n";
    let scene = read(text.as_bytes()).unwrap();
    assert_eq!(scene.triangle_count(None), 3);
    assert_eq!(scene.bounds.size(), DVec3::new(2., 2., 0.));
    assert!(
        scene
            .warnings
            .iter()
            .any(|w| w.key == yo3mfs::i18n::Key::MeshAttributesWarning)
    );
}

#[test]
fn every_binary_truncation_is_rejected() {
    for bytes in [
        include_bytes!("fixtures/sample-little.ply").as_slice(),
        include_bytes!("fixtures/sample-big.ply").as_slice(),
    ] {
        for end in 0..bytes.len() {
            assert!(read(&bytes[..end]).is_err(), "accepted truncation at {end}");
        }
        let mut trailing = bytes.to_vec();
        trailing.push(0);
        assert!(read(&trailing).is_err());
    }
}
