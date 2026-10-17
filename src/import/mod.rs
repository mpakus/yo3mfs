//! Bounded, read-only 3MF import. No extracted files or external URI fetches.
use crate::i18n::{Key, Message};
mod bambu;
mod model;
mod package;
mod xml;
mod zip_index;

use crate::scene::{Bounds, Instance, Scene};
use glam::{DMat4, DVec3};
use model::{Model, Reference};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::{Read, Seek},
    path::Path,
    sync::atomic::{AtomicBool, AtomicU8, Ordering},
};

pub const CORE: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
pub const PRODUCTION: &str = "http://schemas.microsoft.com/3dmanufacturing/production/2015/06";
pub const MODEL_REL: &str = "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";

#[derive(Debug, thiserror::Error)]
#[error("{}", self.localized(crate::i18n::Language::English))]
pub enum ImportError {
    Io(#[from] std::io::Error),
    Zip(#[from] zip::result::ZipError),
    Invalid(Message),
    Unsupported(String),
    Limit(Key),
    Cancelled,
    Empty,
}
impl ImportError {
    pub fn localized(&self, language: crate::i18n::Language) -> String {
        let (key, detail) = match self {
            Self::Io(e) => (Key::IoError, e.to_string()),
            Self::Zip(e) => (Key::ZipError, e.to_string()),
            Self::Invalid(m) => (Key::Invalid3mf, m.render(language)),
            Self::Unsupported(ns) => (Key::Unsupported, ns.clone()),
            Self::Limit(key) => (Key::LimitExceeded, language.text(*key).into()),
            Self::Cancelled => return language.text(Key::Cancelled).into(),
            Self::Empty => return language.text(Key::NoGeometry).into(),
        };
        Message::with(key, [detail]).render(language)
    }
}

#[derive(Clone, Debug)]
pub struct Limits {
    pub archive_bytes: u64,
    pub entries: usize,
    pub xml_bytes: u64,
    pub expanded_bytes: u64,
    pub memory_bytes: usize,
    pub triangles: usize,
    pub rendered_triangles: usize,
    pub instances: usize,
    pub xml_depth: usize,
    pub component_depth: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            archive_bytes: 512 * 1024 * 1024,
            entries: 10_000,
            xml_bytes: 512 * 1024 * 1024,
            expanded_bytes: 2 * 1024 * 1024 * 1024,
            memory_bytes: 2 * 1024 * 1024 * 1024,
            triangles: 5_000_000,
            rendered_triangles: 10_000_000,
            instances: 100_000,
            xml_depth: 128,
            component_depth: 64,
        }
    }
}
pub(crate) fn check_cancel(cancel: &AtomicBool) -> Result<(), ImportError> {
    if cancel.load(Ordering::Relaxed) {
        Err(ImportError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn invalid(message: impl Into<Message>) -> ImportError {
    ImportError::Invalid(message.into())
}

pub fn load(path: &Path, limits: &Limits, cancel: &AtomicBool) -> Result<Scene, ImportError> {
    load_with_progress(path, limits, cancel, &AtomicU8::new(0))
}
pub fn load_with_progress(
    path: &Path,
    limits: &Limits,
    cancel: &AtomicBool,
    progress: &AtomicU8,
) -> Result<Scene, ImportError> {
    progress.store(1, Ordering::Relaxed);
    let f = File::open(path)?;
    if !f.metadata()?.is_file() {
        return Err(invalid(Key::RegularFile));
    }
    read_archive_progress(f, limits, cancel, progress)
}

pub fn read_archive<R: Read + Seek>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    read_archive_progress(input, limits, cancel, &AtomicU8::new(0))
}
fn read_archive_progress<R: Read + Seek>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
    progress: &AtomicU8,
) -> Result<Scene, ImportError> {
    check_cancel(cancel)?;
    let mut package = package::Package::new(input, limits, cancel)?;
    let roots = package.relationships("")?;
    if roots.is_empty() {
        return Err(ImportError::Empty);
    }
    if roots.len() != 1 {
        return Err(invalid(Key::RootCount));
    }
    progress.store(2, Ordering::Relaxed);
    let root = roots[0].clone();
    let mut models = HashMap::new();
    let mut pending = VecDeque::from([root.clone()]);
    let mut queued = HashSet::from([root.clone()]);
    let mut budget = model::Budget::default();
    budget.charge(package.index_bytes, limits)?;
    let mut warnings = Vec::new();
    while let Some(part) = pending.pop_front() {
        check_cancel(cancel)?;
        let m = package.model(&part, &mut budget, &mut warnings)?;
        if part == root && !m.has_build {
            return Err(invalid(Key::RootBuild));
        }
        let related = package.relationships(&part)?;
        for r in m.references() {
            if let Some(target) = &r.part {
                if part != root {
                    return Err(invalid(Key::ProductionPath));
                }
                if !related.contains(target) {
                    return Err(invalid(Message::with(Key::MissingRelationship, [target])));
                }
                if queued.insert(target.clone()) {
                    pending.push_back(target.clone());
                }
            }
        }
        models.insert(part, m);
    }
    let metadata = package.bambu(&mut warnings, &mut budget)?;
    progress.store(3, Ordering::Relaxed);
    let mut scene = Scene {
        meshes: vec![],
        instances: vec![],
        bounds: Bounds {
            min: DVec3::ZERO,
            max: DVec3::ZERO,
        },
        warnings,
        summaries: Default::default(),
    };
    let mut resolver = Resolver {
        models: &mut models,
        root: &root,
        scene: &mut scene,
        mesh_ids: HashMap::new(),
        active: HashSet::new(),
        limits,
        cancel,
        metadata: &metadata,
        rendered: 0,
        visits: 0,
        excluded: 0,
        bytes: budget.bytes,
    };
    let root_model = resolver
        .models
        .get(&root)
        .ok_or_else(|| invalid(Key::MissingRoot))?;
    let build_bytes = root_model
        .build
        .iter()
        .map(|r| std::mem::size_of::<Reference>() + r.part.as_ref().map_or(0, String::len))
        .sum::<usize>();
    resolver.bytes = resolver.bytes.saturating_add(build_bytes);
    if resolver.bytes > limits.memory_bytes {
        return Err(ImportError::Limit(Key::BuildMemory));
    }
    let build = root_model.build.clone();
    let scale = root_model.unit;
    let mut occurrences = HashMap::new();
    for item in &build {
        let index = occurrences
            .entry((item.part.clone(), item.object))
            .or_insert(0u32);
        let plate = metadata.plates.get(&(item.object, *index)).copied();
        *index += 1;
        resolver.expand(
            &root,
            item,
            DMat4::from_scale(DVec3::splat(scale)),
            0,
            plate,
            None,
        )?;
    }
    if resolver.excluded > 0 {
        resolver
            .scene
            .warnings
            .push(Message::with(Key::ExcludedWarning, [resolver.excluded]));
    }
    if !metadata.plates.is_empty() && resolver.scene.instances.iter().any(|i| i.plate.is_none()) {
        for i in &mut resolver.scene.instances {
            i.plate = None;
        }
        resolver.scene.warnings.push(Key::PlatesWarning.into());
    }
    scene.compute_summaries(|| check_cancel(cancel))?;
    scene.bounds = scene.bounds_for(None).ok_or(ImportError::Empty)?;
    if !scene.bounds.size().is_finite()
        || scene.bounds.radius() <= 0.
        || scene.bounds.radius() > 1e15
    {
        return Err(invalid(Key::BoundsRange));
    }
    scene.warnings.sort();
    scene.warnings.dedup();
    Ok(scene)
}

struct Resolver<'a> {
    models: &'a mut HashMap<String, Model>,
    root: &'a str,
    scene: &'a mut Scene,
    mesh_ids: HashMap<(String, u32), usize>,
    active: HashSet<(String, u32)>,
    limits: &'a Limits,
    cancel: &'a AtomicBool,
    metadata: &'a bambu::Metadata,
    rendered: usize,
    visits: usize,
    excluded: usize,
    bytes: usize,
}
impl Resolver<'_> {
    fn expand(
        &mut self,
        source: &str,
        r: &Reference,
        parent: DMat4,
        depth: usize,
        plate: Option<u32>,
        role: Option<String>,
    ) -> Result<(), ImportError> {
        check_cancel(self.cancel)?;
        self.visits += 1;
        if depth > self.limits.component_depth
            || self.visits > self.limits.instances.saturating_mul(8)
        {
            return Err(ImportError::Limit(Key::ComponentCount));
        }
        let part = r.part.as_deref().unwrap_or(source);
        let key = (part.to_string(), r.object);
        if !self.active.insert(key.clone()) {
            return Err(invalid(Key::ComponentCycle));
        }
        let source_unit = self
            .models
            .get(source)
            .ok_or_else(|| invalid(Key::MissingModelPart))?
            .unit;
        let m = self
            .models
            .get_mut(part)
            .ok_or_else(|| invalid(Message::with(Key::MissingPart, [part])))?;
        let conversion = if part != source {
            m.unit / source_unit
        } else {
            1.
        };
        let world = parent * r.transform * DMat4::from_scale(DVec3::splat(conversion));
        if !world.is_finite() || world.determinant().abs() < 1e-30 {
            return Err(invalid(Key::SingularTransform));
        }
        let object = m.objects.get_mut(&r.object).ok_or_else(|| {
            invalid(Message::with(
                Key::MissingObject,
                [r.object.to_string(), part.to_string()],
            ))
        })?;
        let object_role = role
            .or_else(|| {
                if part == self.root {
                    self.metadata.roles.get(&r.object).cloned()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| object.kind.clone());
        if matches!(
            object_role.as_str(),
            "negative_part"
                | "modifier_part"
                | "support_enforcer"
                | "support_blocker"
                | "other"
                | "support"
                | "solidsupport"
        ) {
            self.excluded += 1;
        } else {
            if !matches!(object_role.as_str(), "model" | "normal_part" | "") {
                let warning: Message = Key::UnknownRole.into();
                if !self.scene.warnings.iter().any(|w| w == &warning) {
                    self.bytes = self
                        .bytes
                        .saturating_add(std::mem::size_of::<Message>() + 64);
                    if self.bytes > self.limits.memory_bytes {
                        return Err(ImportError::Limit(Key::WarningMemory));
                    }
                    self.scene.warnings.push(warning);
                }
            }
            self.bytes = self
                .bytes
                .saturating_add(512 + object.name.as_ref().map_or(0, String::len));
            let temporary = object
                .components
                .iter()
                .map(|c| 256 + c.part.as_ref().map_or(0, String::len))
                .sum::<usize>();
            self.bytes = self.bytes.saturating_add(temporary);
            if self.bytes > self.limits.memory_bytes {
                return Err(ImportError::Limit(Key::InstanceMemory));
            }
            let name = object
                .name
                .clone()
                .unwrap_or_else(|| format!("Object {}", r.object));
            let components = object.components.clone();
            let mesh_id = if let Some(id) = self.mesh_ids.get(&key) {
                Some(*id)
            } else if let Some(mesh) = object.mesh.take() {
                let id = self.scene.meshes.len();
                self.scene.meshes.push(mesh);
                self.mesh_ids.insert(key.clone(), id);
                Some(id)
            } else {
                None
            };
            if let Some(mesh) = mesh_id {
                if self.scene.instances.len() >= self.limits.instances {
                    return Err(ImportError::Limit(Key::InstanceCount));
                }
                self.rendered = self
                    .rendered
                    .checked_add(self.scene.meshes[mesh].triangles.len())
                    .ok_or(ImportError::Limit(Key::TriangleCount))?;
                if self.rendered > self.limits.rendered_triangles {
                    return Err(ImportError::Limit(Key::RenderedTriangles));
                }
                for (n, p) in self.scene.meshes[mesh].vertices.iter().enumerate() {
                    if n % 4096 == 0 {
                        check_cancel(self.cancel)?;
                    }
                    let p = world.transform_point3(*p);
                    if !p.is_finite() || p.abs().max_element() > 1e15 {
                        return Err(invalid(Key::CoordinateRange));
                    }
                }
                self.scene.instances.push(Instance {
                    mesh,
                    transform: world,
                    name,
                    plate,
                });
            }
            for c in components {
                let child_role = if part == self.root {
                    self.metadata.parts.get(&(r.object, c.object)).cloned()
                } else {
                    None
                };
                self.expand(part, &c, world, depth + 1, plate, child_role)?;
            }
            self.bytes = self.bytes.saturating_sub(temporary);
        }
        self.active.remove(&key);
        Ok(())
    }
}
