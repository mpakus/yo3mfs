//! Bounded, read-only 3MF/STL/STEP import. No extracted files or external URI fetches.
use crate::i18n::{Key, Message};
mod bambu;
mod gltf;
mod mesh;
mod model;
mod obj;
mod package;
pub(crate) mod paint;
mod ply;
mod step;
mod stl;
mod xml;
mod zip_index;

use crate::scene::{Bounds, Instance, Scene};
use glam::{DMat4, DVec3};
use model::{Model, Reference};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::{Read, Seek, SeekFrom},
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
    Stl(Message),
    Model(Message),
    Obj(Message),
    Ply(Message),
    Gltf(Message),
    Step(Message),
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
            Self::Stl(m) => (Key::InvalidStl, m.render(language)),
            Self::Model(m) => (Key::InvalidModel, m.render(language)),
            Self::Obj(m) => (Key::InvalidObj, m.render(language)),
            Self::Ply(m) => (Key::InvalidPly, m.render(language)),
            Self::Gltf(m) => (Key::InvalidGltf, m.render(language)),
            Self::Step(m) => (Key::InvalidStep, m.render(language)),
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
    pub paint_depth: usize,
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
            paint_depth: 64,
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
    check_cancel(cancel)?;
    let mut f = File::open(path)?;
    let metadata = f.metadata()?;
    if !metadata.is_file() {
        return Err(invalid(Key::RegularFile));
    }
    let mut signature = Vec::with_capacity(84);
    f.by_ref().take(84).read_to_end(&mut signature)?;
    f.seek(SeekFrom::Start(0))?;
    if stl::binary_count(&signature, metadata.len()).is_none()
        && matches!(
            signature.get(..4),
            Some(b"PK\x03\x04" | b"PK\x05\x06" | b"PK\x06\x06")
        )
    {
        read_archive_progress(f, limits, cancel, progress)
    } else {
        progress.store(2, Ordering::Relaxed);
        let ascii_stl = signature.trim_ascii_start().starts_with(b"solid");
        let is_step = signature.trim_ascii_start().starts_with(b"ISO-10303-21;")
            || path.extension().is_some_and(|ext| {
                ext.eq_ignore_ascii_case("step") || ext.eq_ignore_ascii_case("stp")
            });
        let ext = path.extension();
        let extension = |name| ext.is_some_and(|e| e.eq_ignore_ascii_case(name));
        let scene = if stl::binary_count(&signature, metadata.len()).is_some() || ascii_stl {
            read_stl(f, limits, cancel)?
        } else if signature.trim_ascii_start().starts_with(b"ISO-10303-21;") {
            read_step(f, limits, cancel)?
        } else if signature.starts_with(b"glTF") || signature.trim_ascii_start().starts_with(b"{") {
            read_gltf(f, limits, cancel)?
        } else if signature.starts_with(b"ply\n") || signature.starts_with(b"ply\r\n") {
            read_ply(f, limits, cancel)?
        } else if extension("obj") || obj::signature(&signature) {
            read_obj(f, limits, cancel)?
        } else if extension("gltf") || extension("glb") {
            read_gltf(f, limits, cancel)?
        } else if extension("ply") {
            read_ply(f, limits, cancel)?
        } else if is_step {
            read_step(f, limits, cancel)?
        } else {
            read_stl(f, limits, cancel)?
        };
        progress.store(3, Ordering::Relaxed);
        Ok(scene)
    }
}

pub fn read_ply<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    ply::read(input, limits, cancel).map_err(|e| match e {
        ImportError::Model(m) => ImportError::Ply(m),
        other => other,
    })
}

pub fn read_gltf<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    gltf::read(input, limits, cancel).map_err(|e| match e {
        ImportError::Model(m) => ImportError::Gltf(m),
        other => other,
    })
}

pub fn read_obj<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    obj::read(input, limits, cancel).map_err(|e| match e {
        ImportError::Model(m) => ImportError::Obj(m),
        other => other,
    })
}

pub fn read_step<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    step::read(input, limits, cancel)
}

pub fn read_stl<R: Read + Seek>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    stl::read(input, limits, cancel)
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
        palette: metadata.palette.clone(),
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
            InstanceStyle {
                plate,
                role: None,
                filament: metadata
                    .object_filaments
                    .get(&item.object)
                    .copied()
                    .unwrap_or(if metadata.palette.is_empty() { 0 } else { 1 }),
            },
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
struct InstanceStyle {
    plate: Option<u32>,
    role: Option<String>,
    filament: u16,
}
impl Resolver<'_> {
    fn expand(
        &mut self,
        source: &str,
        r: &Reference,
        parent: DMat4,
        depth: usize,
        style: InstanceStyle,
    ) -> Result<(), ImportError> {
        let InstanceStyle {
            plate,
            role,
            filament,
        } = style;
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
                    filament,
                });
            }
            for c in components {
                let child_role = if part == self.root {
                    self.metadata.parts.get(&(r.object, c.object)).cloned()
                } else {
                    None
                };
                let child_filament = if part == self.root {
                    self.metadata
                        .part_filaments
                        .get(&(r.object, c.object))
                        .copied()
                        .or_else(|| {
                            (c.part.as_deref().unwrap_or(part) == self.root)
                                .then(|| self.metadata.object_filaments.get(&c.object).copied())
                                .flatten()
                        })
                        .unwrap_or(filament)
                } else {
                    filament
                };
                self.expand(
                    part,
                    &c,
                    world,
                    depth + 1,
                    InstanceStyle {
                        plate,
                        role: child_role,
                        filament: child_filament,
                    },
                )?;
            }
            self.bytes = self.bytes.saturating_sub(temporary);
        }
        self.active.remove(&key);
        Ok(())
    }
}
