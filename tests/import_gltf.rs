use base64::Engine;
use glam::DVec3;
use serde_json::{Value, json};
use std::{io::Write, sync::atomic::AtomicBool};
use yo3mfs::{
    import::{ImportError, Limits, load},
    scene::Scene,
};

fn document() -> (Value, Vec<u8>) {
    let bytes: Vec<_> = [0f32, 0., 0., 1., 0., 0., 0., 1., 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .collect();
    (
        json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":36}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],"materials":[{"pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1]}}],"nodes":[{"mesh":0},{"translation":[2,0,0],"children":[2]},{"mesh":0}],"scenes":[{"nodes":[0,1]}],"scene":0}),
        bytes,
    )
}
fn embedded(mut doc: Value, bytes: &[u8]) -> Vec<u8> {
    doc["buffers"][0]["uri"] = format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    )
    .into();
    serde_json::to_vec(&doc).unwrap()
}
fn glb(doc: Value, bytes: &[u8]) -> Vec<u8> {
    let mut json = serde_json::to_vec(&doc).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = bytes.to_vec();
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let mut out = b"glTF".to_vec();
    out.extend(2u32.to_le_bytes());
    out.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    out.extend((json.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(b"BIN\0");
    out.extend(bin);
    out
}
fn read(bytes: &[u8], limits: &Limits) -> Result<Scene, ImportError> {
    let mut file = tempfile::Builder::new().suffix(".gltf").tempfile().unwrap();
    file.write_all(bytes).unwrap();
    load(file.path(), limits, &AtomicBool::new(false))
}
#[test]
fn glb_and_embedded_gltf_preserve_shared_instances_units_axes_and_base_color() {
    let (doc, bytes) = document();
    for file in [embedded(doc.clone(), &bytes), glb(doc, &bytes)] {
        let scene = read(&file, &Limits::default()).unwrap();
        assert_eq!(scene.meshes.len(), 1);
        assert_eq!(scene.instances.len(), 2);
        assert_eq!(scene.triangle_count(None), 2);
        assert!((scene.bounds.size() - DVec3::new(3000., 0., 1000.)).length() < 1e-8);
        let prepared =
            yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
        assert!(
            prepared.vertices[0]
                .iter()
                .all(|v| v.color == yo3mfs::scene::packed_color([255, 0, 0]))
        );
    }
}
#[test]
fn rejects_external_buffers_cycles_invalid_accessors_and_required_extensions() {
    let (doc, bytes) = document();
    let mut external = doc.clone();
    external["buffers"][0]["uri"] = "../../private.bin".into();
    assert!(read(&serde_json::to_vec(&external).unwrap(), &Limits::default()).is_err());
    let mut variants = Vec::new();
    let mut wrapped = doc.clone();
    wrapped["meshes"][0]["primitives"][0]["attributes"]["POSITION"] = 4294967296u64.into();
    variants.push(wrapped);
    let mut cycle = doc.clone();
    cycle["nodes"][2]["children"] = json!([1]);
    variants.push(cycle);
    let mut count = doc.clone();
    count["accessors"][0]["count"] = 0.into();
    variants.push(count);
    let mut stride = doc.clone();
    stride["bufferViews"][0]["byteStride"] = 4.into();
    variants.push(stride);
    let mut ext = doc.clone();
    ext["extensionsRequired"] = json!(["KHR_draco_mesh_compression"]);
    variants.push(ext);
    let mut offset = doc.clone();
    offset["accessors"][0]["byteOffset"] = u64::MAX.into();
    variants.push(offset);
    for doc in variants {
        assert!(read(&embedded(doc, &bytes), &Limits::default()).is_err());
    }
}

#[test]
fn triangle_strip_connectors_are_skipped_and_count_against_source_budget() {
    let (mut doc, _) = document();
    let points = [
        [0f32, 0., 0.],
        [1., 0., 0.],
        [0., 1., 0.],
        [0., 1., 0.],
        [2., 0., 0.],
        [2., 0., 0.],
        [3., 0., 0.],
        [2., 1., 0.],
    ];
    let bytes: Vec<_> = points
        .into_iter()
        .flatten()
        .flat_map(f32::to_le_bytes)
        .collect();
    doc["buffers"][0]["byteLength"] = bytes.len().into();
    doc["bufferViews"][0]["byteLength"] = bytes.len().into();
    doc["accessors"][0]["count"] = 8.into();
    doc["accessors"][0]["max"] = json!([3, 1, 0]);
    doc["meshes"][0]["primitives"][0]["mode"] = 5.into();
    let file = embedded(doc, &bytes);
    let scene = read(&file, &Limits::default()).unwrap();
    assert_eq!(scene.triangle_count(None), 4);
    assert!(
        scene
            .warnings
            .iter()
            .any(|m| m.key == yo3mfs::i18n::Key::DegenerateWarning)
    );
    assert!(matches!(
        read(
            &file,
            &Limits {
                triangles: 5,
                ..Limits::default()
            }
        ),
        Err(ImportError::Limit(_))
    ));
}
#[test]
fn budgets_and_glb_lengths_are_enforced() {
    let (doc, bytes) = document();
    let file = glb(doc, &bytes);
    for limits in [
        Limits {
            instances: 1,
            ..Limits::default()
        },
        Limits {
            rendered_triangles: 1,
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
    ] {
        assert!(matches!(read(&file, &limits), Err(ImportError::Limit(_))));
    }
    for end in [0, 4, 12, file.len() - 1] {
        assert!(read(&file[..end], &Limits::default()).is_err());
    }
    let mut extra = file;
    extra.push(0);
    assert!(read(&extra, &Limits::default()).is_err());
}

#[test]
fn sparse_positions_and_interleaved_normalized_colors_are_supported() {
    let (mut doc, positions) = document();
    let mut bytes = vec![0, 1, 2, 0];
    bytes.extend(positions);
    doc["buffers"][0]["byteLength"] = bytes.len().into();
    doc["bufferViews"] =
        json!([{"buffer":0,"byteLength":3},{"buffer":0,"byteOffset":4,"byteLength":36}]);
    doc["accessors"][0]
        .as_object_mut()
        .unwrap()
        .remove("bufferView");
    doc["accessors"][0]["sparse"] = json!({"count":3,"indices":{"bufferView":0,"componentType":5121},"values":{"bufferView":1}});
    let scene = read(&embedded(doc.clone(), &bytes), &Limits::default()).unwrap();
    assert_eq!(scene.triangle_count(None), 2);
    for indices in [[0, 0, 2], [0, 2, 1], [0, 1, 3]] {
        bytes[..3].copy_from_slice(&indices);
        assert!(read(&embedded(doc.clone(), &bytes), &Limits::default()).is_err());
    }

    let (mut doc, positions) = document();
    let mut bytes = Vec::new();
    for (i, p) in positions.chunks_exact(12).enumerate() {
        bytes.extend_from_slice(p);
        let mut rgba = [0, 0, 0, 255];
        rgba[i] = 255;
        bytes.extend(rgba);
    }
    doc["buffers"][0]["byteLength"] = 48.into();
    doc["bufferViews"][0]["byteLength"] = 48.into();
    doc["bufferViews"][0]["byteStride"] = 16.into();
    doc["accessors"].as_array_mut().unwrap().push(json!({"bufferView":0,"byteOffset":12,"componentType":5121,"count":3,"type":"VEC4","normalized":true}));
    doc["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"] = 1.into();
    doc["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"] = json!([1, 1, 1, 1]);
    let scene = read(&embedded(doc, &bytes), &Limits::default()).unwrap();
    assert_eq!(
        scene.meshes[0].colors,
        [[255, 0, 0], [0, 255, 0], [0, 0, 255]].map(yo3mfs::scene::packed_color)
    );
}

#[test]
fn reflects_and_scales_instances_and_rejects_deformation_or_singular_transforms() {
    let (mut doc, bytes) = document();
    doc["nodes"][0]["scale"] = json!([-2, 3, 1]);
    doc["scenes"][0]["nodes"] = json!([0]);
    let scene = read(&embedded(doc.clone(), &bytes), &Limits::default()).unwrap();
    assert_eq!(scene.bounds.min, DVec3::new(-2000., 0., 0.));
    assert_eq!(scene.bounds.size(), DVec3::new(2000., 0., 3000.));
    for (field, value) in [
        ("scale", json!([0, 1, 1])),
        ("rotation", json!([0, 0, 0, 0])),
        ("weights", json!([1])),
    ] {
        let mut invalid = doc.clone();
        invalid["nodes"][0][field] = value;
        assert!(read(&embedded(invalid, &bytes), &Limits::default()).is_err());
    }
    doc["meshes"][0]["primitives"][0]["targets"] = json!([{"POSITION":0}]);
    assert!(read(&embedded(doc, &bytes), &Limits::default()).is_err());
}

#[test]
fn fully_degenerate_primitives_do_not_break_other_geometry_preparation() {
    let (mut doc, mut bytes) = document();
    bytes.extend([0, 0, 0]);
    doc["buffers"][0]["byteLength"] = 39.into();
    doc["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":36,"byteLength":3}));
    doc["accessors"]
        .as_array_mut()
        .unwrap()
        .push(json!({"bufferView":1,"componentType":5121,"count":3,"type":"SCALAR"}));
    let empty = json!({"attributes":{"POSITION":0},"indices":1});
    doc["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(empty.clone());
    let scene = read(&embedded(doc.clone(), &bytes), &Limits::default()).unwrap();
    let prepared =
        yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false)).unwrap();
    assert_eq!(prepared.scene.meshes.len(), 1);
    assert_eq!(prepared.scene.instances.len(), 2);
    assert_eq!(prepared.rendered_triangles, 2);
    assert!(
        prepared
            .scene
            .warnings
            .iter()
            .any(|w| w.key == yo3mfs::i18n::Key::DegenerateWarning)
    );
    doc["meshes"][0]["primitives"] = json!([empty]);
    assert!(matches!(
        read(&embedded(doc, &bytes), &Limits::default()),
        Err(ImportError::Empty)
    ));
}
