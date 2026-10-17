use crate::i18n::Key;
use crate::{
    import::{ImportError, Limits, check_cancel},
    scene::Scene,
};
use glam::{DMat4, DVec3};
use std::sync::atomic::AtomicBool;

pub struct Prepared {
    pub scene: Scene,
    pub vertices: Vec<Vec<[f32; 3]>>,
    pub instances: Vec<PreparedInstance>,
    pub gpu_bytes: usize,
}
pub struct PreparedInstance {
    pub mesh: usize,
    pub plate: Option<u32>,
    pub matrix: [[f32; 4]; 4],
}

pub fn prepare(
    scene: Scene,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Prepared, ImportError> {
    let gpu_bytes = scene
        .meshes
        .iter()
        .map(|m| m.vertices.len() * 12 + m.triangles.len() * 12)
        .sum::<usize>()
        + scene.instances.len() * 64;
    // Account for retained CPU data, conversion staging and GPU buffers together.
    if scene
        .estimated_bytes()
        .saturating_add(gpu_bytes.saturating_mul(2))
        > limits.memory_bytes
    {
        return Err(ImportError::Limit(Key::SceneMemory));
    }
    let mut vertices = Vec::new();
    let mut origins = Vec::new();
    for mesh in &scene.meshes {
        check_cancel(cancel)?;
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
            .try_reserve_exact(mesh.vertices.len())
            .map_err(|_| ImportError::Limit(Key::VertexBuffer))?;
        for (n, p) in mesh.vertices.iter().enumerate() {
            if n % 4096 == 0 {
                check_cancel(cancel)?;
            }
            points.push((*p - origin).as_vec3().to_array());
        }
        vertices.push(points);
    }
    let normalize = DMat4::from_scale(DVec3::splat(1. / scene.bounds.radius()))
        * DMat4::from_translation(-scene.bounds.center());
    let mut instances = Vec::new();
    for i in &scene.instances {
        check_cancel(cancel)?;
        let matrix = (normalize * i.transform * DMat4::from_translation(origins[i.mesh])).as_mat4();
        if !matrix.is_finite() {
            return Err(ImportError::Invalid(Key::GpuMatrixRange.into()));
        }
        instances.push(PreparedInstance {
            mesh: i.mesh,
            plate: i.plate,
            matrix: matrix.to_cols_array_2d(),
        });
    }
    instances.sort_by_key(|i| (i.mesh, i.plate));
    Ok(Prepared {
        scene,
        vertices,
        instances,
        gpu_bytes,
    })
}
