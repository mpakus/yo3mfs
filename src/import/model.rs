use super::{CORE, ImportError, Limits, PRODUCTION, invalid, package, xml::Tag};
use crate::i18n::{Key, Message};
use crate::scene::{FacetPaint, Mesh};
use glam::{DMat4, DVec3};
use std::{collections::HashMap, sync::atomic::AtomicBool};

#[derive(Clone, Debug)]
pub struct Reference {
    pub object: u32,
    pub part: Option<String>,
    pub transform: DMat4,
}
#[derive(Debug)]
pub struct Object {
    pub kind: String,
    pub name: Option<String>,
    pub mesh: Option<Mesh>,
    pub components: Vec<Reference>,
}
pub struct Model {
    pub unit: f64,
    pub objects: HashMap<u32, Object>,
    pub build: Vec<Reference>,
    pub has_build: bool,
}
impl Model {
    pub fn references(&self) -> impl Iterator<Item = &Reference> {
        self.build
            .iter()
            .chain(self.objects.values().flat_map(|o| o.components.iter()))
    }
}
#[derive(Default)]
pub struct Budget {
    pub bytes: usize,
    triangles: usize,
    expanded_triangles: usize,
}
impl Budget {
    pub fn charge(&mut self, bytes: usize, limits: &Limits) -> Result<(), ImportError> {
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ImportError::Limit(Key::GeometryMemory))?;
        if self.bytes > limits.memory_bytes {
            return Err(ImportError::Limit(Key::GeometryMemory));
        }
        Ok(())
    }
}
pub struct Parser<'a> {
    part: &'a str,
    cancel: &'a AtomicBool,
    limits: Limits,
    budget: &'a mut Budget,
    warnings: &'a mut Vec<Message>,
    model: Model,
    current: Option<(u32, Object)>,
    visual_warning: bool,
    paint_warning: bool,
    build_seen: bool,
    resources_seen: bool,
    skipped: usize,
}
impl<'a> Parser<'a> {
    pub fn new(
        part: &'a str,
        cancel: &'a AtomicBool,
        limits: Limits,
        budget: &'a mut Budget,
        warnings: &'a mut Vec<Message>,
    ) -> Self {
        Self {
            part,
            cancel,
            limits,
            budget,
            warnings,
            model: Model {
                unit: 1.,
                objects: HashMap::new(),
                build: vec![],
                has_build: false,
            },
            current: None,
            visual_warning: false,
            paint_warning: false,
            build_seen: false,
            resources_seen: false,
            skipped: 0,
        }
    }
    pub fn event(
        &mut self,
        t: &Tag,
        path: &[(String, String)],
        end: bool,
    ) -> Result<(), ImportError> {
        if path.len() == 1 && !end {
            if t.name != "model" || t.ns != CORE {
                return Err(invalid(Key::ExpectedCore));
            }
            self.model.unit = match t.attr("unit").unwrap_or("millimeter") {
                "micron" => 0.001,
                "millimeter" => 1.,
                "centimeter" => 10.,
                "inch" => 25.4,
                "foot" => 304.8,
                "meter" => 1000.,
                other => return Err(invalid(Message::with(Key::UnknownUnit, [other]))),
            };
            for ns in &t.required {
                if ns != PRODUCTION {
                    return Err(ImportError::Unsupported(ns.clone()));
                }
            }
            if t.attr("recommendedextensions").is_some() {
                self.visual_warning = true;
            }
        }
        if !end
            && (t.ns != CORE
                || t.attrs.keys().any(|(ns, key)| {
                    (!ns.is_empty()
                        && ns != PRODUCTION
                        && ns != "http://www.w3.org/XML/1998/namespace")
                        || matches!(key.as_str(), "pid" | "p1" | "p2" | "p3" | "displaycolor")
                }))
        {
            self.visual_warning = true;
        }
        if path.iter().any(|(ns, _)| ns != CORE) {
            return Ok(());
        }
        let names = path
            .iter()
            .map(|(_, name)| name.as_str())
            .collect::<Vec<_>>();
        match (names.as_slice(), end) {
            (["model", "resources"], false) => {
                if self.resources_seen {
                    return Err(invalid(Key::DuplicateResources));
                }
                self.resources_seen = true;
            }
            (["model", "build"], false) => {
                if self.build_seen {
                    return Err(invalid(Key::DuplicateBuild));
                }
                self.build_seen = true;
                self.model.has_build = true;
            }
            (["model", "resources", "basematerials"], false) => {
                self.visual_warning = true;
            }
            (["model", "resources", "object"], false) => {
                self.budget.charge(
                    512 + t.attr("name").map_or(0, str::len) + t.attr("type").map_or(0, str::len),
                    &self.limits,
                )?;
                if self.current.is_some() {
                    return Err(invalid(Key::NestedObject));
                }
                self.current = Some((
                    t.id("id")?,
                    Object {
                        kind: t.attr("type").unwrap_or("model").into(),
                        name: t.attr("name").map(str::to_string),
                        mesh: None,
                        components: vec![],
                    },
                ));
            }
            (["model", "resources", "object", "mesh"], false) => {
                let o = self
                    .current
                    .as_mut()
                    .ok_or_else(|| invalid(Key::MeshOutsideObject))?;
                if o.1.mesh.is_some() || !o.1.components.is_empty() {
                    return Err(invalid(Key::MultipleMeshes));
                }
                o.1.mesh = Some(Mesh {
                    vertices: vec![],
                    triangles: vec![],
                    painting: vec![],
                });
            }
            (["model", "resources", "object", "mesh", "vertices", "vertex"], false) => {
                self.budget.charge(48, &self.limits)?;
                let p = DVec3::new(
                    number(t.need("x")?)?,
                    number(t.need("y")?)?,
                    number(t.need("z")?)?,
                );
                let mesh = self.mesh()?;
                if mesh.vertices.len() >= u32::MAX as usize {
                    return Err(ImportError::Limit(Key::VertexCount));
                }
                mesh.vertices
                    .try_reserve(1)
                    .map_err(|_| ImportError::Limit(Key::VertexMemory))?;
                mesh.vertices.push(p);
            }
            (
                [
                    "model",
                    "resources",
                    "object",
                    "mesh",
                    "triangles",
                    "triangle",
                ],
                false,
            ) => {
                self.budget.triangles += 1;
                if self.budget.triangles > self.limits.triangles {
                    return Err(ImportError::Limit(Key::TriangleCount));
                }
                self.budget.charge(24, &self.limits)?;
                let mut tri = [0u32; 3];
                for (i, key) in ["v1", "v2", "v3"].iter().enumerate() {
                    tri[i] = t.need(key)?.parse().map_err(|_| invalid(Key::BadIndex))?;
                }
                let mut paint = None;
                if let Some(code) = t.attr("paint_color").filter(|s| !s.is_empty()) {
                    match super::paint::visit(
                        code,
                        [DVec3::ZERO, DVec3::X, DVec3::Y],
                        &self.limits,
                        self.cancel,
                        |_, _| {},
                    ) {
                        Ok(leaves) => {
                            self.budget.charge(
                                code.len() * 2 + std::mem::size_of::<FacetPaint>() * 2,
                                &self.limits,
                            )?;
                            paint = Some(FacetPaint {
                                triangle: 0,
                                code: code.to_owned(),
                                leaves,
                            });
                        }
                        Err(ImportError::Invalid(_)) => self.paint_warning = true,
                        Err(error) => return Err(error),
                    }
                }
                self.budget.expanded_triangles = self
                    .budget
                    .expanded_triangles
                    .saturating_add(paint.as_ref().map_or(1, |p| p.leaves));
                if self.budget.expanded_triangles > self.limits.rendered_triangles {
                    return Err(ImportError::Limit(Key::RenderedTriangles));
                }
                let mesh = self.mesh()?;
                if let Some(mut paint) = paint {
                    paint.triangle = mesh.triangles.len();
                    mesh.painting
                        .try_reserve(1)
                        .map_err(|_| ImportError::Limit(Key::TriangleMemory))?;
                    mesh.painting.push(paint);
                }
                mesh.triangles
                    .try_reserve(1)
                    .map_err(|_| ImportError::Limit(Key::TriangleMemory))?;
                mesh.triangles.push(tri);
            }
            (["model", "resources", "object", "components", "component"], false) => {
                self.budget.charge(
                    512 + t.attr_ns(PRODUCTION, "path").map_or(0, str::len),
                    &self.limits,
                )?;
                let reference = self.reference(t)?;
                let o = self
                    .current
                    .as_mut()
                    .ok_or_else(|| invalid(Key::ComponentOutside))?;
                if o.1.mesh.is_some() {
                    return Err(invalid(Key::MixedComponents));
                }
                o.1.components.push(reference);
            }
            (["model", "build", "item"], false) => {
                self.budget.charge(
                    512 + t.attr_ns(PRODUCTION, "path").map_or(0, str::len),
                    &self.limits,
                )?;
                if self.model.build.len() >= self.limits.instances {
                    return Err(ImportError::Limit(Key::BuildCount));
                }
                self.model.build.push(self.reference(t)?);
            }
            (["model", "resources", "object"], true) => {
                let (id, mut object) = self
                    .current
                    .take()
                    .ok_or_else(|| invalid(Key::UnexpectedObject))?;
                if let Some(mesh) = object.mesh.as_mut() {
                    for (n, tri) in mesh.triangles.iter().enumerate() {
                        if n % 4096 == 0 {
                            super::check_cancel(self.cancel)?;
                        }
                        if tri.iter().any(|&v| v as usize >= mesh.vertices.len()) {
                            return Err(invalid(Key::IndexOutside));
                        }
                    }
                    let before = mesh.triangles.len();
                    let mut kept = 0;
                    let mut paint_cursor = 0;
                    let mut paints_kept = 0;
                    for n in 0..mesh.triangles.len() {
                        if n % 4096 == 0 {
                            super::check_cancel(self.cancel)?;
                        }
                        let t = mesh.triangles[n];
                        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                        let keep = (b - a).cross(c - a).length_squared() > 0.;
                        if mesh
                            .painting
                            .get(paint_cursor)
                            .is_some_and(|p| p.triangle == n)
                        {
                            if keep {
                                mesh.painting[paint_cursor].triangle = kept;
                                mesh.painting.swap(paints_kept, paint_cursor);
                                paints_kept += 1;
                            }
                            paint_cursor += 1;
                        }
                        if keep {
                            mesh.triangles[kept] = t;
                            kept += 1;
                        }
                    }
                    mesh.triangles.truncate(kept);
                    mesh.painting.truncate(paints_kept);
                    self.skipped += before - mesh.triangles.len();
                    if mesh.triangles.is_empty() {
                        return Err(ImportError::Empty);
                    }
                    // Compact unused vertices, including vertices referenced only by discarded faces.
                    // This bounds subsequent instance work by the rendered-triangle limit.
                    self.budget.charge(mesh.vertices.len() * 4, &self.limits)?;
                    let mut remap = vec![u32::MAX; mesh.vertices.len()];
                    for (n, t) in mesh.triangles.iter().enumerate() {
                        if n % 4096 == 0 {
                            super::check_cancel(self.cancel)?;
                        }
                        for &v in t {
                            remap[v as usize] = 0;
                        }
                    }
                    let mut next = 0u32;
                    for (old, index) in remap.iter_mut().enumerate() {
                        if old % 4096 == 0 {
                            super::check_cancel(self.cancel)?;
                        }
                        if *index != u32::MAX {
                            mesh.vertices[next as usize] = mesh.vertices[old];
                            *index = next;
                            next += 1;
                        }
                    }
                    mesh.vertices.truncate(next as usize);
                    for (n, t) in mesh.triangles.iter_mut().enumerate() {
                        if n % 4096 == 0 {
                            super::check_cancel(self.cancel)?;
                        }
                        for v in t {
                            *v = remap[*v as usize];
                        }
                    }
                } else if object.components.is_empty() {
                    return Err(invalid(Key::EmptyObject));
                }
                if self.model.objects.insert(id, object).is_some() {
                    return Err(invalid(Key::DuplicateObject));
                }
            }
            _ => {}
        }
        Ok(())
    }
    fn mesh(&mut self) -> Result<&mut Mesh, ImportError> {
        self.current
            .as_mut()
            .and_then(|(_, o)| o.mesh.as_mut())
            .ok_or_else(|| invalid(Key::GeometryOutside))
    }
    fn reference(&self, t: &Tag) -> Result<Reference, ImportError> {
        let part = t
            .attr_ns(PRODUCTION, "path")
            .map(|p| package::resolve(self.part, p))
            .transpose()?;
        Ok(Reference {
            object: t.id("objectid")?,
            part,
            transform: transform(t.attr("transform"))?,
        })
    }
    pub fn finish(self) -> Result<Model, ImportError> {
        if !self.resources_seen {
            return Err(invalid(Key::MissingResources));
        }
        if self.visual_warning {
            self.warnings.push(Key::VisualWarning.into());
        }
        if self.paint_warning {
            self.warnings.push(Key::PaintWarning.into());
        }
        if self.skipped > 0 {
            self.warnings
                .push(Message::with(Key::DegenerateWarning, [self.skipped]));
        }
        Ok(self.model)
    }
}
fn number(s: &str) -> Result<f64, ImportError> {
    let n = s.parse::<f64>().map_err(|_| invalid(Key::BadNumber))?;
    if !n.is_finite() || n.abs() > 1e15 {
        return Err(invalid(Key::NumberRange));
    }
    Ok(n)
}
fn transform(text: Option<&str>) -> Result<DMat4, ImportError> {
    let Some(text) = text else {
        return Ok(DMat4::IDENTITY);
    };
    let values = text
        .split_whitespace()
        .map(number)
        .collect::<Result<Vec<_>, _>>()?;
    let v: [f64; 12] = values.try_into().map_err(|_| invalid(Key::MatrixSize))?;
    // 3MF uses row vectors; transposition into glam's column-vector convention.
    Ok(DMat4::from_cols_array(&[
        v[0], v[1], v[2], 0., v[3], v[4], v[5], 0., v[6], v[7], v[8], 0., v[9], v[10], v[11], 1.,
    ]))
}
