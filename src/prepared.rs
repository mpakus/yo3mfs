use crate::i18n::Key;
use crate::{
    import::{ImportError, Limits, check_cancel, paint},
    scene::{NEUTRAL, Scene, packed_color},
};
use glam::{DMat4, DVec3};
use std::sync::atomic::AtomicBool;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "desktop", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Vertex {
    pub position: [f32; 3],
    /// Packed sRGB, or zero to inherit the instance's project filament.
    pub color: u32,
}
pub struct Prepared {
    pub scene: Scene,
    pub vertices: Vec<Vec<Vertex>>,
    /// Unpainted meshes reuse the original index buffer without a CPU copy.
    pub indices: Vec<Option<Vec<[u32; 3]>>>,
    pub instances: Vec<PreparedInstance>,
    pub gpu_bytes: usize,
    pub rendered_triangles: usize,
}
pub struct PreparedInstance {
    pub mesh: usize,
    pub plate: Option<u32>,
    pub matrix: [[f32; 4]; 4],
    pub color: u32,
}
fn color(filament: u16, palette: &[Option<[u8; 3]>], warning: &mut bool) -> u32 {
    if filament == 0 {
        return 0;
    }
    match palette.get(usize::from(filament) - 1).copied().flatten() {
        Some(rgb) => packed_color(rgb),
        None => {
            *warning = true;
            0
        }
    }
}
pub fn prepare(
    mut scene: Scene,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Prepared, ImportError> {
    check_cancel(cancel)?;
    let counts = scene
        .meshes
        .iter()
        .map(|m| {
            let leaves = m.painting.iter().map(|p| p.leaves).sum::<usize>();
            (
                m.vertices.len().saturating_add(leaves.saturating_mul(3)),
                m.triangles
                    .len()
                    .saturating_sub(m.painting.len())
                    .saturating_add(leaves),
            )
        })
        .collect::<Vec<_>>();
    let rendered_triangles = scene
        .instances
        .iter()
        .fold(0usize, |total, i| total.saturating_add(counts[i.mesh].1));
    if rendered_triangles > limits.rendered_triangles {
        return Err(ImportError::Limit(Key::RenderedTriangles));
    }
    let gpu_bytes = counts
        .iter()
        .fold(0usize, |total, (points, triangles)| {
            total
                .saturating_add(points.saturating_mul(16))
                .saturating_add(triangles.saturating_mul(12))
        })
        .saturating_add(scene.instances.len().saturating_mul(68));
    // Retained geometry, conversion staging, upload staging and GPU allocations.
    let peak_bytes = scene
        .estimated_bytes()
        .saturating_add(gpu_bytes.saturating_mul(2))
        .saturating_add(
            scene
                .instances
                .len()
                .saturating_mul(std::mem::size_of::<PreparedInstance>()),
        );
    if peak_bytes > limits.memory_bytes {
        return Err(ImportError::Limit(Key::SceneMemory));
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut origins = Vec::new();
    let mut palette_warning = false;
    for (mesh, &(point_count, triangle_count)) in scene.meshes.iter().zip(&counts) {
        check_cancel(cancel)?;
        if point_count > u32::MAX as usize {
            return Err(ImportError::Limit(Key::VertexCount));
        }
        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        for (n, p) in mesh.vertices.iter().enumerate() {
            if n % 4096 == 0 {
                check_cancel(cancel)?;
            }
            min = min.min(*p);
            max = max.max(*p);
        }
        let origin = min + (max - min) * 0.5;
        origins.push(origin);
        let mut points = Vec::new();
        points
            .try_reserve_exact(point_count)
            .map_err(|_| ImportError::Limit(Key::VertexBuffer))?;
        for (n, p) in mesh.vertices.iter().enumerate() {
            if n % 4096 == 0 {
                check_cancel(cancel)?;
            }
            points.push(Vertex {
                position: (*p - origin).as_vec3().to_array(),
                color: 0,
            });
        }
        let painted_indices = if mesh.painting.is_empty() {
            None
        } else {
            let mut faces = Vec::new();
            faces
                .try_reserve_exact(triangle_count)
                .map_err(|_| ImportError::Limit(Key::TriangleMemory))?;
            let mut painting = mesh.painting.iter().peekable();
            for (n, triangle) in mesh.triangles.iter().enumerate() {
                if n % 4096 == 0 {
                    check_cancel(cancel)?;
                }
                if let Some(painted) = painting.peek().filter(|p| p.triangle == n) {
                    let local = triangle.map(|i| mesh.vertices[i as usize] - origin);
                    let leaves =
                        paint::visit(&painted.code, local, limits, cancel, |corners, filament| {
                            let tint = color(filament, &scene.palette, &mut palette_warning);
                            let first = points.len() as u32;
                            points.extend(corners.map(|p| Vertex {
                                position: p.as_vec3().to_array(),
                                color: tint,
                            }));
                            faces.push([first, first + 1, first + 2]);
                        })?;
                    if leaves != painted.leaves {
                        return Err(ImportError::Invalid(Key::BadPaint.into()));
                    }
                    painting.next();
                } else {
                    faces.push(*triangle);
                }
            }
            Some(faces)
        };
        vertices.push(points);
        indices.push(painted_indices);
    }
    let normalize = DMat4::from_scale(DVec3::splat(1. / scene.bounds.radius()))
        * DMat4::from_translation(-scene.bounds.center());
    let mut instances = Vec::new();
    instances
        .try_reserve_exact(scene.instances.len())
        .map_err(|_| ImportError::Limit(Key::InstanceMemory))?;
    for i in &scene.instances {
        check_cancel(cancel)?;
        let matrix = (normalize * i.transform * DMat4::from_translation(origins[i.mesh])).as_mat4();
        if !matrix.is_finite() {
            return Err(ImportError::Invalid(Key::GpuMatrixRange.into()));
        }
        let tint = color(i.filament, &scene.palette, &mut palette_warning);
        instances.push(PreparedInstance {
            mesh: i.mesh,
            plate: i.plate,
            matrix: matrix.to_cols_array_2d(),
            color: if tint == 0 {
                packed_color(NEUTRAL)
            } else {
                tint
            },
        });
    }
    if palette_warning && !scene.warnings.contains(&Key::PaletteWarning.into()) {
        scene.warnings.push(Key::PaletteWarning.into());
    }
    instances.sort_by_key(|i| (i.mesh, i.plate));
    Ok(Prepared {
        scene,
        vertices,
        indices,
        instances,
        gpu_bytes,
        rendered_triangles,
    })
}
