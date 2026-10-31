//! Self-contained glTF 2.0 / GLB static scenes. No resource URI is opened.
mod validation;
use super::{
    ImportError, Limits, check_cancel,
    mesh::{self, Budget, bad},
};
use crate::{
    i18n::Key,
    scene::{Instance, Mesh, Scene, packed_color},
};
use base64::Engine;
use glam::{DMat4, DQuat, DVec3};
use gltf::{
    Document,
    accessor::{DataType, Dimensions},
    buffer::Source,
    mesh::{Mode, Semantic},
};
use std::{borrow::Cow, collections::HashMap, io::Read, sync::atomic::AtomicBool};
const INPUT: u64 = 64 * 1024 * 1024;
const JSON: usize = 16 * 1024 * 1024;

fn u32_at(bytes: &[u8], offset: usize) -> Result<u32, ImportError> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(|| bad(Key::GltfStructure))?
            .try_into()
            .map_err(|_| bad(Key::GltfStructure))?,
    ))
}
fn container(bytes: &[u8]) -> Result<(&[u8], Option<&[u8]>), ImportError> {
    if !bytes.starts_with(b"glTF") {
        return Ok((bytes, None));
    }
    if u32_at(bytes, 4)? != 2 || u32_at(bytes, 8)? as usize != bytes.len() {
        return Err(bad(Key::GltfStructure));
    }
    let mut offset = 12usize;
    let mut json = None;
    let mut bin = None;
    while offset < bytes.len() {
        let len = u32_at(bytes, offset)? as usize;
        let kind = u32_at(bytes, offset + 4)?;
        if !len.is_multiple_of(4) {
            return Err(bad(Key::GltfStructure));
        }
        let end = offset
            .checked_add(8)
            .and_then(|v| v.checked_add(len))
            .ok_or_else(|| bad(Key::GltfStructure))?;
        let chunk = bytes
            .get(offset + 8..end)
            .ok_or_else(|| bad(Key::GltfStructure))?;
        match kind {
            0x4e4f534a if offset == 12 => json = Some(chunk),
            0x004e4942 if json.is_some() && bin.is_none() => bin = Some(chunk),
            0x4e4f534a | 0x004e4942 => return Err(bad(Key::GltfStructure)),
            _ if json.is_none() => return Err(bad(Key::GltfStructure)),
            _ => {} // The GLB spec permits unknown chunks after JSON.
        }
        offset = end;
    }
    Ok((json.ok_or_else(|| bad(Key::GltfStructure))?, bin))
}
/// Reserve for the typed JSON tree and validation scratch before serde allocates.
fn json_budget(bytes: &[u8], budget: &mut Budget<'_>) -> Result<(), ImportError> {
    if bytes.len() > JSON {
        return Err(ImportError::Limit(Key::GltfInputSize));
    }
    budget.charge(bytes.len().saturating_mul(4))?;
    let (mut quoted, mut escaped) = (false, false);
    for (i, &c) in bytes.iter().enumerate() {
        if i % 8192 == 0 {
            check_cancel(budget.cancel)?;
        }
        if quoted {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                quoted = false;
            }
        } else {
            match c {
                b'"' => quoted = true,
                b'{' => budget.charge(2048)?,
                b'[' => budget.charge(512)?,
                b',' => budget.charge(256)?,
                _ => {}
            }
        }
    }
    Ok(())
}
pub(super) fn read<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    let mut budget = Budget::new(limits, cancel)?;
    let bytes = mesh::read_bytes(
        input,
        &mut budget,
        INPUT.min(limits.archive_bytes),
        Key::GltfInputSize,
    )?;
    let (json, bin) = container(&bytes)?;
    json_budget(json, &mut budget)?;
    let raw: serde_json::Value =
        serde_json::from_slice(json).map_err(|_| bad(Key::GltfStructure))?;
    validation::json_indices(&raw, "", cancel)?;
    let root: gltf::json::Root =
        serde_json::from_value(raw).map_err(|_| bad(Key::GltfStructure))?;
    check_cancel(cancel)?;
    if root.asset.version != "2.0"
        || root
            .asset
            .min_version
            .as_deref()
            .is_some_and(|v| v != "2.0")
    {
        return Err(bad(Key::GltfStructure));
    }
    if let Some(required) = root.extensions_required.first() {
        return Err(ImportError::Unsupported(required.clone()));
    }
    let doc = Document::from_json(root).map_err(|_| bad(Key::GltfStructure))?;
    let mut buffers: Vec<Cow<'_, [u8]>> = vec![];
    let mut decoded = 0usize;
    for buffer in doc.buffers() {
        check_cancel(cancel)?;
        decoded = decoded.saturating_add(buffer.length());
        if decoded as u64 > INPUT || decoded as u64 > limits.expanded_bytes {
            return Err(ImportError::Limit(Key::GltfInputSize));
        }
        let data = match buffer.source() {
            Source::Bin => {
                if buffer.index() != 0 {
                    return Err(bad(Key::GltfStructure));
                }
                let bin = bin.ok_or_else(|| bad(Key::GltfStructure))?;
                if bin.len() < buffer.length() || bin.len() - buffer.length() > 3 {
                    return Err(bad(Key::GltfStructure));
                }
                Cow::Borrowed(&bin[..buffer.length()])
            }
            Source::Uri(uri) => {
                let data = uri
                    .strip_prefix("data:application/octet-stream;base64,")
                    .or_else(|| uri.strip_prefix("data:application/gltf-buffer;base64,"))
                    .ok_or_else(|| bad(Key::GltfExternal))?;
                if data.len() > ((buffer.length().saturating_add(2)) / 3).saturating_mul(4) {
                    return Err(bad(Key::GltfStructure));
                }
                let mut out = Vec::new();
                budget.reserve(&mut out, buffer.length().saturating_add(3))?;
                base64::engine::general_purpose::STANDARD
                    .decode_vec(data, &mut out)
                    .map_err(|_| bad(Key::GltfStructure))?;
                if out.len() != buffer.length() {
                    return Err(bad(Key::GltfStructure));
                }
                Cow::Owned(out)
            }
        };
        budget.reserve(&mut buffers, 1)?;
        buffers.push(data);
    }
    validation::accessors(&doc, &buffers, &budget)?;
    let mut scene = mesh::scene(mesh::empty_mesh());
    scene.meshes.clear();
    scene.instances.clear();
    scene.warnings.push(Key::GltfPreviewWarning.into());
    if doc.extensions_used().next().is_some() {
        scene.warnings.push(Key::GltfExtensionsWarning.into());
    }
    if doc.scenes().len() > 1 {
        scene.warnings.push(Key::GltfSceneWarning.into());
    }
    let selected = doc
        .default_scene()
        .or_else(|| doc.scenes().next())
        .ok_or(ImportError::Empty)?;
    if doc.nodes().len() > limits.instances.saturating_mul(8) {
        return Err(ImportError::Limit(Key::ComponentCount));
    }
    // Exact right-handed basis conversion: (x,y,z) meters -> (x,-z,y) millimeters.
    let basis = DMat4::from_cols_array(&[
        1000., 0., 0., 0., 0., 0., 1000., 0., 0., -1000., 0., 0., 0., 0., 0., 1.,
    ]);
    let mut walker = Walker {
        buffers: &buffers,
        budget: &mut budget,
        scene: &mut scene,
        meshes: HashMap::new(),
        visited: vec![false; doc.nodes().len()],
    };
    for node in selected.nodes() {
        walker.node(node, basis, 0)?;
    }
    mesh::finish(scene, &mut budget)
}

struct Walker<'a, 'b, 'c> {
    buffers: &'a [Cow<'a, [u8]>],
    budget: &'b mut Budget<'c>,
    scene: &'b mut Scene,
    meshes: HashMap<(usize, usize), usize>,
    visited: Vec<bool>,
}
impl Walker<'_, '_, '_> {
    fn node(
        &mut self,
        node: gltf::Node<'_>,
        parent: DMat4,
        depth: usize,
    ) -> Result<(), ImportError> {
        check_cancel(self.budget.cancel)?;
        if depth > self.budget.limits.component_depth {
            return Err(ImportError::Limit(Key::ComponentCount));
        }
        if self.visited[node.index()] {
            return Err(bad(Key::ComponentCycle));
        }
        self.visited[node.index()] = true;
        if node.skin().is_some() || node.weights().is_some() {
            return Err(bad(Key::GltfDeformation));
        }
        let raw = &node.clone().transform();
        let local = match raw {
            gltf::scene::Transform::Matrix { matrix } => {
                let m = DMat4::from_cols_array_2d(&matrix.map(|c| c.map(f64::from)));
                if m.x_axis.w != 0. || m.y_axis.w != 0. || m.z_axis.w != 0. || m.w_axis.w != 1. {
                    return Err(bad(Key::GltfStructure));
                }
                m
            }
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => {
                let q = DQuat::from_array(rotation.map(f64::from));
                if !q.is_finite() || (q.length_squared() - 1.).abs() > 1e-5 {
                    return Err(bad(Key::GltfStructure));
                }
                DMat4::from_scale_rotation_translation(
                    DVec3::from_array(scale.map(f64::from)),
                    q,
                    DVec3::from_array(translation.map(f64::from)),
                )
            }
        };
        let world = parent * local;
        if !world.is_finite() || world.determinant().abs() < 1e-30 {
            return Err(bad(Key::SingularTransform));
        }
        if let Some(mesh) = node.mesh() {
            for primitive in mesh.primitives() {
                if self.scene.instances.len() >= self.budget.limits.instances {
                    return Err(ImportError::Limit(Key::InstanceCount));
                }
                let key = (mesh.index(), primitive.index());
                let id = if let Some(&id) = self.meshes.get(&key) {
                    id
                } else {
                    let geometry = primitive_mesh(primitive, self.buffers, self.budget)?;
                    let id = self.scene.meshes.len();
                    self.budget.reserve(&mut self.scene.meshes, 1)?;
                    self.scene.meshes.push(geometry);
                    self.budget.charge(128)?;
                    self.meshes.insert(key, id);
                    id
                };
                let name = node.name().or_else(|| mesh.name()).unwrap_or("");
                self.budget.charge(name.len())?;
                self.budget.reserve(&mut self.scene.instances, 1)?;
                self.scene.instances.push(Instance {
                    mesh: id,
                    transform: world,
                    name: name.to_owned(),
                    plate: None,
                    filament: 0,
                });
            }
        }
        for child in node.children() {
            self.node(child, world, depth + 1)?;
        }
        Ok(())
    }
}
fn primitive_mesh(
    primitive: gltf::Primitive<'_>,
    buffers: &[Cow<'_, [u8]>],
    budget: &mut Budget<'_>,
) -> Result<Mesh, ImportError> {
    if primitive.morph_targets().next().is_some() {
        return Err(bad(Key::GltfDeformation));
    }
    if !matches!(
        primitive.mode(),
        Mode::Triangles | Mode::TriangleStrip | Mode::TriangleFan
    ) {
        return Err(bad(Key::MeshUnsupported));
    }
    let positions = primitive
        .get(&Semantic::Positions)
        .ok_or_else(|| bad(Key::GltfStructure))?;
    if positions.data_type() != DataType::F32
        || positions.dimensions() != Dimensions::Vec3
        || positions.normalized()
    {
        return Err(bad(Key::GltfStructure));
    }
    if positions.count() > budget.limits.triangles.saturating_mul(3) {
        return Err(ImportError::Limit(Key::VertexCount));
    }
    let reader = primitive.reader(|b| buffers.get(b.index()).map(|b| b.as_ref()));
    let mut mesh = mesh::empty_mesh();
    for p in reader
        .read_positions()
        .ok_or_else(|| bad(Key::GltfStructure))?
    {
        budget.vertex(&mut mesh, DVec3::from_array(p.map(f64::from)))?;
    }
    let base = primitive
        .material()
        .pbr_metallic_roughness()
        .base_color_factor();
    if base
        .iter()
        .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
    {
        return Err(bad(Key::GltfStructure));
    }
    budget.reserve(&mut mesh.colors, mesh.vertices.len())?;
    if let Some(colors) = primitive.get(&Semantic::Colors(0)) {
        if colors.count() != mesh.vertices.len()
            || !matches!(colors.dimensions(), Dimensions::Vec3 | Dimensions::Vec4)
            || !matches!(
                colors.data_type(),
                DataType::F32 | DataType::U8 | DataType::U16
            )
            || (colors.data_type() != DataType::F32 && !colors.normalized())
        {
            return Err(bad(Key::GltfStructure));
        }
        for rgba in reader
            .read_colors(0)
            .ok_or_else(|| bad(Key::GltfStructure))?
            .into_rgba_f32()
        {
            check_cancel(budget.cancel)?;
            if rgba
                .iter()
                .any(|v| !v.is_finite() || !(0. ..=1.).contains(v))
            {
                return Err(bad(Key::GltfStructure));
            }
            mesh.colors.push(linear_color([
                rgba[0] * base[0],
                rgba[1] * base[1],
                rgba[2] * base[2],
            ]));
        }
    } else {
        mesh.colors.resize(
            mesh.vertices.len(),
            linear_color([base[0], base[1], base[2]]),
        );
    }
    let mut indices = Vec::new();
    if let Some(a) = primitive.indices() {
        if a.dimensions() != Dimensions::Scalar
            || !matches!(a.data_type(), DataType::U8 | DataType::U16 | DataType::U32)
            || a.normalized()
        {
            return Err(bad(Key::GltfStructure));
        }
        if a.count() > budget.limits.triangles.saturating_mul(3) {
            return Err(ImportError::Limit(Key::TriangleCount));
        }
        budget.reserve(&mut indices, a.count())?;
        for i in reader
            .read_indices()
            .ok_or_else(|| bad(Key::GltfStructure))?
            .into_u32()
        {
            check_cancel(budget.cancel)?;
            indices.push(i);
        }
    } else {
        budget.reserve(&mut indices, mesh.vertices.len())?;
        indices.extend(0..mesh.vertices.len() as u32);
    }
    if indices.len() < 3 {
        return Err(bad(Key::GltfStructure));
    }
    match primitive.mode() {
        Mode::Triangles => {
            if !indices.len().is_multiple_of(3) {
                return Err(bad(Key::GltfStructure));
            }
            for t in indices.chunks_exact(3) {
                budget.triangle_or_skip(&mut mesh, [t[0], t[1], t[2]])?;
            }
        }
        Mode::TriangleStrip => {
            for i in 2..indices.len() {
                let t = if i % 2 == 0 {
                    [indices[i - 2], indices[i - 1], indices[i]]
                } else {
                    [indices[i - 1], indices[i - 2], indices[i]]
                };
                budget.triangle_or_skip(&mut mesh, t)?;
            }
        }
        Mode::TriangleFan => {
            for i in 2..indices.len() {
                budget.triangle_or_skip(&mut mesh, [indices[0], indices[i - 1], indices[i]])?;
            }
        }
        _ => return Err(bad(Key::MeshUnsupported)),
    }
    Ok(mesh)
}
fn linear_color(rgb: [f32; 3]) -> u32 {
    packed_color(rgb.map(|v| {
        let s = if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        };
        (s.clamp(0., 1.) * 255.).round() as u8
    }))
}
