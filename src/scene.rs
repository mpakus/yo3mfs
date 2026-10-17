//! Geometry is shared by instances; all world-space measurements are millimeters.
use glam::{DMat4, DVec3};
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<DVec3>,
    pub triangles: Vec<[u32; 3]>,
}

#[derive(Debug)]
pub struct Instance {
    pub mesh: usize,
    pub transform: DMat4,
    pub name: String,
    pub plate: Option<u32>,
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: DVec3,
    pub max: DVec3,
}

impl Bounds {
    pub fn size(self) -> DVec3 {
        self.max - self.min
    }
    pub fn center(self) -> DVec3 {
        self.min + self.size() * 0.5
    }
    pub fn radius(self) -> f64 {
        self.size().length() * 0.5
    }
}

#[derive(Debug)]
pub struct Scene {
    pub meshes: Vec<Mesh>,
    pub instances: Vec<Instance>,
    pub bounds: Bounds,
    pub warnings: Vec<crate::i18n::Message>,
    pub summaries: BTreeMap<Option<u32>, Summary>,
}

#[derive(Debug, Clone, Copy)]
pub struct Summary {
    pub bounds: Bounds,
    pub instances: usize,
    pub triangles: usize,
}

impl Scene {
    pub fn triangle_count(&self, plate: Option<u32>) -> usize {
        self.summaries.get(&plate).map_or(0, |s| s.triangles)
    }
    pub fn bounds_for(&self, plate: Option<u32>) -> Option<Bounds> {
        self.summaries.get(&plate).map(|s| s.bounds)
    }
    pub fn compute_summaries<E>(
        &mut self,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.summaries.clear();
        for i in &self.instances {
            checkpoint()?;
            let mut min = DVec3::splat(f64::INFINITY);
            let mut max = DVec3::splat(f64::NEG_INFINITY);
            for (n, point) in self.meshes[i.mesh].vertices.iter().enumerate() {
                if n % 4096 == 0 {
                    checkpoint()?;
                }
                let world = i.transform.transform_point3(*point);
                min = min.min(world);
                max = max.max(world);
            }
            for key in std::iter::once(None).chain(i.plate.map(Some)) {
                let summary = self.summaries.entry(key).or_insert(Summary {
                    bounds: Bounds { min, max },
                    instances: 0,
                    triangles: 0,
                });
                summary.bounds.min = summary.bounds.min.min(min);
                summary.bounds.max = summary.bounds.max.max(max);
                summary.instances += 1;
                summary.triangles += self.meshes[i.mesh].triangles.len();
            }
        }
        Ok(())
    }
    pub fn estimated_bytes(&self) -> usize {
        self.meshes
            .iter()
            .map(|m| m.vertices.capacity() * 24 + m.triangles.capacity() * 12)
            .sum::<usize>()
            + self
                .instances
                .iter()
                .map(|i| i.name.capacity())
                .sum::<usize>()
            + self.instances.capacity() * std::mem::size_of::<Instance>()
            + self.meshes.capacity() * std::mem::size_of::<Mesh>()
            + self
                .warnings
                .iter()
                .map(|s| s.estimated_bytes())
                .sum::<usize>()
            + self.warnings.capacity() * std::mem::size_of::<crate::i18n::Message>()
            + self.summaries.len() * (std::mem::size_of::<Summary>() + 64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_apply_instance_transform_and_plate_filter() {
        let mut scene = Scene {
            meshes: vec![Mesh {
                vertices: vec![DVec3::ZERO, DVec3::ONE],
                triangles: vec![[0, 1, 1]],
            }],
            instances: vec![Instance {
                mesh: 0,
                transform: DMat4::from_translation(DVec3::new(5., 2., 3.)),
                name: String::new(),
                plate: Some(1),
            }],
            bounds: Bounds {
                min: DVec3::ZERO,
                max: DVec3::ONE,
            },
            warnings: vec![],
            summaries: BTreeMap::new(),
        };
        scene.compute_summaries(|| Ok::<_, ()>(())).unwrap();
        let b = scene.bounds_for(Some(1)).unwrap();
        assert_eq!(b.min, DVec3::new(5., 2., 3.));
        assert_eq!(b.max, DVec3::new(6., 3., 4.));
        assert!(scene.bounds_for(Some(2)).is_none());
        assert_eq!(scene.triangle_count(None), 1);
    }
}
