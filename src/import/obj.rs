//! Polygonal Wavefront OBJ; material libraries and all external references stay unopened.
use super::{
    ImportError, Limits,
    mesh::{self, Budget, Lines, bad},
};
use crate::{
    i18n::Key,
    scene::{Scene, packed_color},
};
use glam::DVec3;
use std::{io::Read, sync::atomic::AtomicBool};

fn number(s: &str) -> Result<f64, ImportError> {
    s.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .ok_or_else(|| bad(Key::MeshSyntax))
}
fn index(s: &str, len: usize) -> Result<u32, ImportError> {
    let value = s.parse::<i64>().map_err(|_| bad(Key::MeshSyntax))?;
    let i = if value < 0 {
        (len as i64).checked_add(value)
    } else {
        value.checked_sub(1)
    };
    i.filter(|&i| i >= 0 && (i as usize) < len)
        .and_then(|i| u32::try_from(i).ok())
        .ok_or_else(|| bad(Key::MeshSyntax))
}
pub(super) fn read<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    let mut budget = Budget::new(limits, cancel)?;
    let mut lines = Lines::new(input);
    let mut mesh = mesh::empty_mesh();
    let (mut uv, mut normals) = (0usize, 0usize);
    let (mut materials, mut attributes) = (false, false);
    while let Some(line) = lines.next(&budget)? {
        let mut words = line.split('#').next().unwrap_or("").split_whitespace();
        let Some(kind) = words.next() else {
            continue;
        };
        match kind {
            "v" => {
                let values = words.take(8).map(number).collect::<Result<Vec<_>, _>>()?;
                if !matches!(values.len(), 3 | 4 | 6) {
                    return Err(bad(Key::MeshSyntax));
                }
                let w = if values.len() == 4 { values[3] } else { 1. };
                if w == 0. {
                    return Err(bad(Key::MeshSyntax));
                }
                budget.vertex(&mut mesh, DVec3::from_slice(&values[..3]) / w)?;
                if values.len() == 6 || !mesh.colors.is_empty() {
                    let count = mesh.vertices.len();
                    let additional = count - mesh.colors.len();
                    budget.reserve(&mut mesh.colors, additional)?;
                    mesh.colors.resize(count, 0);
                    if values.len() == 6 {
                        if values[3..].iter().any(|v| !(0. ..=1.).contains(v)) {
                            return Err(bad(Key::MeshSyntax));
                        }
                        mesh.colors[count - 1] = packed_color(
                            [values[3], values[4], values[5]].map(|v| (v * 255.).round() as u8),
                        );
                    }
                }
            }
            "vt" | "vn" => {
                let values = words.take(8).map(number).collect::<Result<Vec<_>, _>>()?;
                if (kind == "vn" && values.len() != 3)
                    || (kind == "vt" && !(1..=3).contains(&values.len()))
                {
                    return Err(bad(Key::MeshSyntax));
                }
                if kind == "vt" {
                    uv += 1;
                } else {
                    normals += 1;
                }
                attributes = true;
            }
            "f" => {
                let mut face = Vec::new();
                for word in words {
                    if face.len() >= mesh::FACE {
                        return Err(ImportError::Limit(Key::MeshFaceSize));
                    }
                    let mut parts = word.split('/');
                    let i = index(
                        parts.next().ok_or_else(|| bad(Key::MeshSyntax))?,
                        mesh.vertices.len(),
                    )?;
                    if let Some(t) = parts.next() {
                        if !t.is_empty() {
                            index(t, uv)?;
                        } else if !word.contains("//") {
                            return Err(bad(Key::MeshSyntax));
                        }
                    }
                    if let Some(n) = parts.next() {
                        index(n, normals)?;
                    }
                    if parts.next().is_some() {
                        return Err(bad(Key::MeshSyntax));
                    }
                    face.push(i);
                }
                mesh::polygon(&mut mesh, &face, &mut budget)?;
            }
            "o" | "g" | "s" => {}
            "mtllib" | "usemtl" => materials = true,
            // Curves, freeform surfaces, and include/call commands cannot be represented faithfully.
            _ => return Err(bad(Key::MeshUnsupported)),
        }
    }
    let mut scene = mesh::scene(mesh);
    scene.warnings.push(Key::MeshUnitsWarning.into());
    if materials {
        scene.warnings.push(Key::ObjMaterialsWarning.into());
    }
    if attributes {
        scene.warnings.push(Key::MeshAttributesWarning.into());
    }
    mesh::finish(scene, &mut budget)
}

// OBJ has no magic bytes: inspect complete declaration prefixes, ignoring leading comments.
pub(super) fn signature(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok_and(|text| {
        text.lines().any(|line| {
            matches!(
                line.split_whitespace().next(),
                Some("v" | "vn" | "vt" | "f" | "o" | "g" | "mtllib")
            )
        })
    })
}
