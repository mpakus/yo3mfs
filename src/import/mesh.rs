//! Shared bounded allocation and polygon handling for static mesh formats.
use super::{ImportError, Limits, check_cancel};
use crate::{
    i18n::Key,
    scene::{Bounds, Instance, Mesh, Scene},
};
use glam::{DMat4, DVec2, DVec3};
use std::{
    io::{BufRead, BufReader, Read},
    sync::atomic::AtomicBool,
};

pub(super) const LINE: usize = 64 * 1024;
pub(super) const FACE: usize = 1024;
pub(super) fn bad(key: Key) -> ImportError {
    ImportError::Model(key.into())
}
pub(super) fn empty_mesh() -> Mesh {
    Mesh {
        vertices: vec![],
        triangles: vec![],
        painting: vec![],
        colors: vec![],
    }
}
pub(super) fn read_bytes<R: Read>(
    mut input: R,
    budget: &mut Budget<'_>,
    cap: u64,
    key: Key,
) -> Result<Vec<u8>, ImportError> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        check_cancel(budget.cancel)?;
        let n = input.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        if (bytes.len() as u64).saturating_add(n as u64) > cap {
            return Err(ImportError::Limit(key));
        }
        budget.reserve(&mut bytes, n)?;
        bytes.extend_from_slice(&chunk[..n]);
    }
    Ok(bytes)
}
pub(super) struct Budget<'a> {
    pub limits: &'a Limits,
    pub cancel: &'a AtomicBool,
    bytes: usize,
    triangles: usize,
    skipped: usize,
}
impl<'a> Budget<'a> {
    pub fn new(limits: &'a Limits, cancel: &'a AtomicBool) -> Result<Self, ImportError> {
        let mut b = Self {
            limits,
            cancel,
            bytes: 0,
            triangles: 0,
            skipped: 0,
        };
        // Input buffers, bounded polygon scratch space, and scene bookkeeping.
        b.charge(256 * 1024)?;
        Ok(b)
    }
    pub fn charge(&mut self, bytes: usize) -> Result<(), ImportError> {
        check_cancel(self.cancel)?;
        self.bytes = self.bytes.saturating_add(bytes);
        if self.bytes > self.limits.memory_bytes {
            return Err(ImportError::Limit(Key::GeometryMemory));
        }
        Ok(())
    }
    pub fn reserve<T>(
        &mut self,
        values: &mut Vec<T>,
        additional: usize,
    ) -> Result<(), ImportError> {
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or(ImportError::Limit(Key::GeometryMemory))?;
        if needed > values.capacity() {
            let target = needed.max(values.capacity().saturating_mul(2)).max(16);
            // Charge the full new allocation, retaining the old charge for reallocation peaks.
            self.charge(target.saturating_mul(std::mem::size_of::<T>()))?;
            values
                .try_reserve_exact(target - values.len())
                .map_err(|_| ImportError::Limit(Key::GeometryMemory))?;
        }
        Ok(())
    }
    pub fn vertex(&mut self, mesh: &mut Mesh, p: DVec3) -> Result<(), ImportError> {
        if !p.is_finite() || p.abs().max_element() > 1e15 {
            return Err(bad(Key::CoordinateRange));
        }
        if mesh.vertices.len() >= u32::MAX as usize {
            return Err(ImportError::Limit(Key::VertexCount));
        }
        self.reserve(&mut mesh.vertices, 1)?;
        mesh.vertices.push(p);
        Ok(())
    }
    pub fn triangle(&mut self, mesh: &mut Mesh, t: [u32; 3]) -> Result<(), ImportError> {
        if !self.triangle_or_skip(mesh, t)? {
            return Err(bad(Key::MeshPolygon));
        }
        Ok(())
    }
    pub fn triangle_or_skip(&mut self, mesh: &mut Mesh, t: [u32; 3]) -> Result<bool, ImportError> {
        check_cancel(self.cancel)?;
        self.triangles = self.triangles.saturating_add(1);
        if self.triangles > self.limits.triangles {
            return Err(ImportError::Limit(Key::TriangleCount));
        }
        if t.iter().any(|&i| i as usize >= mesh.vertices.len()) {
            return Err(bad(Key::MeshSyntax));
        }
        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
        if (b - a).cross(c - a).length_squared() == 0. {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(false);
        }
        self.reserve(&mut mesh.triangles, 1)?;
        mesh.triangles.push(t);
        Ok(true)
    }
}

pub(super) struct Lines<R> {
    pub reader: BufReader<R>,
    pub line: Vec<u8>,
    bytes: u64,
}
impl<R: Read> Lines<R> {
    pub fn new(input: R) -> Self {
        Self {
            reader: BufReader::new(input),
            line: vec![],
            bytes: 0,
        }
    }
    pub fn next(&mut self, budget: &Budget<'_>) -> Result<Option<&str>, ImportError> {
        check_cancel(budget.cancel)?;
        self.line.clear();
        let n = self
            .reader
            .by_ref()
            .take((LINE + 1) as u64)
            .read_until(b'\n', &mut self.line)?;
        self.bytes = self.bytes.saturating_add(n as u64);
        if self.bytes > budget.limits.archive_bytes {
            return Err(ImportError::Limit(Key::InputSize));
        }
        if n > LINE {
            return Err(ImportError::Limit(Key::MeshLineSize));
        }
        if n == 0 {
            return Ok(None);
        }
        std::str::from_utf8(&self.line)
            .map(Some)
            .map_err(|_| bad(Key::MeshSyntax))
    }
}

pub(super) fn finish(mut scene: Scene, budget: &mut Budget<'_>) -> Result<Scene, ImportError> {
    if scene.meshes.iter().any(|m| m.triangles.is_empty()) {
        let mut map = Vec::new();
        budget.reserve(&mut map, scene.meshes.len())?;
        let mut next = 0;
        for mesh in &scene.meshes {
            check_cancel(budget.cancel)?;
            if mesh.triangles.is_empty() {
                map.push(None);
            } else {
                map.push(Some(next));
                next += 1;
            }
        }
        scene.meshes.retain(|m| !m.triangles.is_empty());
        scene.instances.retain_mut(|instance| {
            if let Some(mesh) = map[instance.mesh] {
                instance.mesh = mesh;
                true
            } else {
                false
            }
        });
    }
    for mesh in &mut scene.meshes {
        // Unreferenced positions are not part of the displayed surface or its dimensions.
        let mut map = Vec::new();
        budget.reserve(&mut map, mesh.vertices.len())?;
        map.resize(mesh.vertices.len(), u32::MAX);
        for (n, t) in mesh.triangles.iter().enumerate() {
            if n % 4096 == 0 {
                check_cancel(budget.cancel)?;
            }
            for &i in t {
                map[i as usize] = 0;
            }
        }
        let mut used = 0;
        for (i, mapped) in map.iter_mut().enumerate() {
            if i % 4096 == 0 {
                check_cancel(budget.cancel)?;
            }
            if *mapped != u32::MAX {
                *mapped = used as u32;
                mesh.vertices[used] = mesh.vertices[i];
                if !mesh.colors.is_empty() {
                    mesh.colors[used] = mesh.colors[i];
                }
                used += 1;
            }
        }
        mesh.vertices.truncate(used);
        if !mesh.colors.is_empty() {
            mesh.colors.truncate(used);
        }
        for (n, t) in mesh.triangles.iter_mut().enumerate() {
            if n % 4096 == 0 {
                check_cancel(budget.cancel)?;
            }
            *t = t.map(|i| map[i as usize]);
        }
    }
    if budget.skipped > 0 {
        scene.warnings.push(crate::i18n::Message::with(
            Key::DegenerateWarning,
            [budget.skipped],
        ));
    }
    if scene.instances.is_empty() || scene.meshes.iter().all(|m| m.triangles.is_empty()) {
        return Err(ImportError::Empty);
    }
    if scene.instances.len() > budget.limits.instances {
        return Err(ImportError::Limit(Key::InstanceCount));
    }
    let mut rendered = 0usize;
    for i in &scene.instances {
        check_cancel(budget.cancel)?;
        rendered = rendered.saturating_add(scene.meshes[i.mesh].triangles.len());
        if rendered > budget.limits.rendered_triangles {
            return Err(ImportError::Limit(Key::RenderedTriangles));
        }
        if !i.transform.is_finite() || i.transform.determinant().abs() < 1e-30 {
            return Err(bad(Key::SingularTransform));
        }
        for (n, p) in scene.meshes[i.mesh].vertices.iter().enumerate() {
            if n % 4096 == 0 {
                check_cancel(budget.cancel)?;
            }
            let p = i.transform.transform_point3(*p);
            if !p.is_finite() || p.abs().max_element() > 1e15 {
                return Err(bad(Key::CoordinateRange));
            }
        }
    }
    scene.compute_summaries(|| check_cancel(budget.cancel))?;
    scene.bounds = scene.bounds_for(None).ok_or(ImportError::Empty)?;
    if !scene.bounds.size().is_finite()
        || scene.bounds.radius() <= 0.
        || scene.bounds.radius() > 1e15
    {
        return Err(bad(Key::BoundsRange));
    }
    scene.warnings.sort();
    scene.warnings.dedup();
    Ok(scene)
}
pub(super) fn scene(mesh: Mesh) -> Scene {
    Scene {
        meshes: vec![mesh],
        instances: vec![Instance {
            mesh: 0,
            transform: DMat4::IDENTITY,
            name: String::new(),
            plate: None,
            filament: 0,
        }],
        bounds: Bounds {
            min: DVec3::ZERO,
            max: DVec3::ZERO,
        },
        warnings: vec![],
        summaries: Default::default(),
        palette: vec![],
    }
}

/// Project to the dominant plane, reject intersections, then clip ears. Winding is retained.
pub(super) fn polygon(
    mesh: &mut Mesh,
    indices: &[u32],
    budget: &mut Budget<'_>,
) -> Result<(), ImportError> {
    let n = indices.len();
    if n > FACE {
        return Err(ImportError::Limit(Key::MeshFaceSize));
    }
    if n < 3 || indices.iter().any(|&i| i as usize >= mesh.vertices.len()) {
        return Err(bad(Key::MeshSyntax));
    }
    if n == 3 {
        return budget.triangle(mesh, [indices[0], indices[1], indices[2]]);
    }
    let origin = mesh.vertices[indices[0] as usize];
    let points: Vec<_> = indices
        .iter()
        .map(|&i| mesh.vertices[i as usize] - origin)
        .collect();
    let normal: DVec3 = (0..n).map(|i| points[i].cross(points[(i + 1) % n])).sum();
    let length = normal.length();
    let scale = points.iter().map(|p| p.length()).fold(0f64, f64::max);
    if length == 0.
        || points
            .iter()
            .any(|p| p.dot(normal / length).abs() > scale * 1e-7)
    {
        return Err(bad(Key::MeshPolygon));
    }
    let a = normal.abs();
    let p: Vec<DVec2> = points
        .iter()
        .map(|v| {
            if a.x >= a.y && a.x >= a.z {
                DVec2::new(v.y, v.z)
            } else if a.y >= a.z {
                DVec2::new(v.z, v.x)
            } else {
                DVec2::new(v.x, v.y)
            }
        })
        .collect();
    let cross = |a: DVec2, b: DVec2, c: DVec2| (b - a).perp_dot(c - a);
    let eps = scale * scale * 1e-12;
    for i in 0..n {
        check_cancel(budget.cancel)?;
        if (p[i] - p[(i + 1) % n]).length_squared() <= eps {
            return Err(bad(Key::MeshPolygon));
        }
        for j in i + 1..n {
            if j == (i + 1) % n || (j + 1) % n == i {
                continue;
            }
            let (a, b, c, d) = (p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]);
            let (ab_c, ab_d, cd_a, cd_b) = (
                cross(a, b, c),
                cross(a, b, d),
                cross(c, d, a),
                cross(c, d, b),
            );
            if ab_c * ab_d <= 0.
                && cd_a * cd_b <= 0.
                && a.min(b).cmple(c.max(d)).all()
                && c.min(d).cmple(a.max(b)).all()
            {
                return Err(bad(Key::MeshPolygon));
            }
        }
    }
    let area: f64 = (0..n).map(|i| p[i].perp_dot(p[(i + 1) % n])).sum();
    let sign = area.signum();
    if area.abs() <= eps {
        return Err(bad(Key::MeshPolygon));
    }
    let mut remaining: Vec<usize> = (0..n).collect();
    while remaining.len() > 3 {
        check_cancel(budget.cancel)?;
        let mut ear = None;
        for k in 0..remaining.len() {
            let a = remaining[(k + remaining.len() - 1) % remaining.len()];
            let b = remaining[k];
            let c = remaining[(k + 1) % remaining.len()];
            if cross(p[a], p[b], p[c]) * sign <= eps {
                continue;
            }
            if remaining.iter().any(|&j| {
                j != a
                    && j != b
                    && j != c
                    && cross(p[a], p[b], p[j]) * sign >= -eps
                    && cross(p[b], p[c], p[j]) * sign >= -eps
                    && cross(p[c], p[a], p[j]) * sign >= -eps
            }) {
                continue;
            }
            ear = Some((k, [indices[a], indices[b], indices[c]]));
            break;
        }
        let (k, t) = ear.ok_or_else(|| bad(Key::MeshPolygon))?;
        budget.triangle(mesh, t)?;
        remaining.remove(k);
    }
    budget.triangle(
        mesh,
        [
            indices[remaining[0]],
            indices[remaining[1]],
            indices[remaining[2]],
        ],
    )
}
