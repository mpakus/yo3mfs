//! Stanford PLY 1.0 polygon meshes, ASCII and both binary byte orders.
use super::{
    ImportError, Limits, check_cancel,
    mesh::{self, Budget, bad},
};
use crate::{
    i18n::Key,
    scene::{Scene, packed_color},
};
use glam::DVec3;
use std::{io::Read, sync::atomic::AtomicBool};

#[derive(Clone, Copy, PartialEq)]
enum Scalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}
impl Scalar {
    fn parse(s: &str) -> Result<Self, ImportError> {
        Ok(match s {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return Err(bad(Key::MeshSyntax)),
        })
    }
    fn integer(self) -> bool {
        !matches!(self, Self::F32 | Self::F64)
    }
    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}
struct Property {
    name: String,
    kind: Scalar,
    count: Option<Scalar>,
}
struct Element {
    name: String,
    count: usize,
    properties: Vec<Property>,
}
#[derive(Clone, Copy, PartialEq)]
enum Encoding {
    Ascii,
    Little,
    Big,
}
struct Data<'a> {
    bytes: &'a [u8],
    position: usize,
    line: usize,
    encoding: Encoding,
}
impl Data<'_> {
    fn header_line(&mut self) -> Result<&str, ImportError> {
        let rest = &self.bytes[self.position..];
        let n = rest
            .iter()
            .position(|&b| b == b'\n')
            .ok_or_else(|| bad(Key::MeshSyntax))?;
        if n >= mesh::LINE {
            return Err(ImportError::Limit(Key::MeshLineSize));
        }
        self.position += n + 1;
        std::str::from_utf8(&rest[..n])
            .map(str::trim)
            .map_err(|_| bad(Key::MeshSyntax))
    }
    fn scalar(&mut self, kind: Scalar) -> Result<f64, ImportError> {
        let value = if self.encoding == Encoding::Ascii {
            while self
                .bytes
                .get(self.position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.advance()?;
            }
            let start = self.position;
            while self
                .bytes
                .get(self.position)
                .is_some_and(|b| !b.is_ascii_whitespace())
            {
                self.advance()?;
            }
            let word = std::str::from_utf8(&self.bytes[start..self.position])
                .map_err(|_| bad(Key::MeshSyntax))?;
            macro_rules! parse {
                ($t:ty) => {
                    word.parse::<$t>()
                        .map(|v| v as f64)
                        .map_err(|_| bad(Key::MeshSyntax))?
                };
            }
            match kind {
                Scalar::I8 => parse!(i8),
                Scalar::U8 => parse!(u8),
                Scalar::I16 => parse!(i16),
                Scalar::U16 => parse!(u16),
                Scalar::I32 => parse!(i32),
                Scalar::U32 => parse!(u32),
                Scalar::F32 => parse!(f32),
                Scalar::F64 => parse!(f64),
            }
        } else {
            let end = self
                .position
                .checked_add(kind.size())
                .ok_or_else(|| bad(Key::MeshSyntax))?;
            let b = self
                .bytes
                .get(self.position..end)
                .ok_or_else(|| bad(Key::MeshSyntax))?;
            self.position = end;
            macro_rules! decode {
                ($t:ty) => {{
                    let a = b.try_into().map_err(|_| bad(Key::MeshSyntax))?;
                    if self.encoding == Encoding::Little {
                        <$t>::from_le_bytes(a) as f64
                    } else {
                        <$t>::from_be_bytes(a) as f64
                    }
                }};
            }
            match kind {
                Scalar::I8 => b[0] as i8 as f64,
                Scalar::U8 => b[0] as f64,
                Scalar::I16 => decode!(i16),
                Scalar::U16 => decode!(u16),
                Scalar::I32 => decode!(i32),
                Scalar::U32 => decode!(u32),
                Scalar::F32 => decode!(f32),
                Scalar::F64 => decode!(f64),
            }
        };
        if !value.is_finite() {
            return Err(bad(Key::MeshSyntax));
        }
        Ok(value)
    }
    fn advance(&mut self) -> Result<(), ImportError> {
        self.line = if self.bytes[self.position] == b'\n' {
            0
        } else {
            self.line + 1
        };
        self.position += 1;
        if self.line > mesh::LINE {
            return Err(ImportError::Limit(Key::MeshLineSize));
        }
        Ok(())
    }
}

pub(super) fn read<R: Read>(
    input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    let mut budget = Budget::new(limits, cancel)?;
    let bytes = mesh::read_bytes(input, &mut budget, limits.archive_bytes, Key::InputSize)?;
    let mut data = Data {
        bytes: &bytes,
        position: 0,
        line: 0,
        encoding: Encoding::Ascii,
    };
    if data.header_line()? != "ply" {
        return Err(bad(Key::MeshSyntax));
    }
    data.encoding = match data.header_line()? {
        "format ascii 1.0" => Encoding::Ascii,
        "format binary_little_endian 1.0" => Encoding::Little,
        "format binary_big_endian 1.0" => Encoding::Big,
        _ => return Err(bad(Key::MeshSyntax)),
    };
    let mut elements: Vec<Element> = vec![];
    loop {
        check_cancel(cancel)?;
        let words: Vec<_> = data.header_line()?.split_whitespace().take(6).collect();
        match words.as_slice() {
            ["end_header"] => break,
            ["comment" | "obj_info", ..] => {}
            ["element", name, count] => {
                if elements.len() >= 256 || name.len() > 128 {
                    return Err(ImportError::Limit(Key::GeometryMemory));
                }
                if elements.iter().any(|e| e.name == *name) {
                    return Err(bad(Key::MeshSyntax));
                }
                let count = count.parse::<usize>().map_err(|_| bad(Key::MeshSyntax))?;
                if count > limits.triangles.saturating_mul(3) {
                    return Err(ImportError::Limit(Key::TriangleCount));
                }
                budget.charge(256)?;
                elements.push(Element {
                    name: name.to_string(),
                    count,
                    properties: vec![],
                });
            }
            ["property", rest @ ..] => {
                let e = elements.last_mut().ok_or_else(|| bad(Key::MeshSyntax))?;
                let (name, kind, count) = match rest {
                    [kind, name] => (*name, Scalar::parse(kind)?, None),
                    ["list", count, kind, name] => {
                        (*name, Scalar::parse(kind)?, Some(Scalar::parse(count)?))
                    }
                    _ => return Err(bad(Key::MeshSyntax)),
                };
                if count.is_some_and(|c| !c.integer())
                    || name.len() > 128
                    || e.properties.iter().any(|p| p.name == name)
                {
                    return Err(bad(Key::MeshSyntax));
                }
                if e.properties.len() >= 256 {
                    return Err(ImportError::Limit(Key::GeometryMemory));
                }
                budget.charge(256)?;
                e.properties.push(Property {
                    name: name.to_string(),
                    kind,
                    count,
                });
            }
            _ => return Err(bad(Key::MeshSyntax)),
        }
        if data.position > 1024 * 1024 {
            return Err(ImportError::Limit(Key::MeshLineSize));
        }
    }
    if data.position > 1024 * 1024 {
        return Err(ImportError::Limit(Key::MeshLineSize));
    }
    let vertex = elements
        .iter()
        .position(|e| e.name == "vertex")
        .ok_or_else(|| bad(Key::MeshSyntax))?;
    let face = elements
        .iter()
        .position(|e| e.name == "face")
        .ok_or_else(|| bad(Key::PlyPoints))?;
    if elements[face].count == 0 {
        return Err(bad(Key::PlyPoints));
    }
    // Vertex data must precede faces so indices are validated without unbounded deferred polygons.
    if vertex > face {
        return Err(bad(Key::MeshUnsupported));
    }
    let vp = &elements[vertex].properties;
    for axis in ["x", "y", "z"] {
        if !vp.iter().any(|p| p.name == axis && p.count.is_none()) {
            return Err(bad(Key::MeshSyntax));
        }
    }
    let colors = ["red", "green", "blue"];
    let colored = colors.iter().all(|name| {
        vp.iter()
            .any(|p| p.name == *name && p.kind == Scalar::U8 && p.count.is_none())
    });
    if !elements[face].properties.iter().any(|p| {
        matches!(p.name.as_str(), "vertex_indices" | "vertex_index")
            && p.count.is_some()
            && p.kind.integer()
    }) {
        return Err(bad(Key::MeshSyntax));
    }
    let mut mesh = mesh::empty_mesh();
    let mut ignored = false;
    for e in &elements {
        if e.properties.is_empty() {
            return Err(bad(Key::MeshSyntax));
        }
        for _ in 0..e.count {
            check_cancel(cancel)?;
            let mut xyz = [0.; 3];
            let mut rgb = [0u8; 3];
            let mut indices = None;
            for p in &e.properties {
                let count = if let Some(t) = p.count {
                    let n = data.scalar(t)?;
                    if n < 0. || n > mesh::FACE as f64 {
                        return Err(ImportError::Limit(Key::MeshFaceSize));
                    }
                    n as usize
                } else {
                    1
                };
                let is_indices = e.name == "face"
                    && matches!(p.name.as_str(), "vertex_indices" | "vertex_index");
                if is_indices && indices.is_some() {
                    return Err(bad(Key::MeshSyntax));
                }
                let mut face_indices = Vec::new();
                for _ in 0..count {
                    let value = data.scalar(p.kind)?;
                    if is_indices {
                        if value < 0. || value > u32::MAX as f64 || !p.kind.integer() {
                            return Err(bad(Key::MeshSyntax));
                        }
                        face_indices.push(value as u32);
                    } else if e.name == "vertex" && p.count.is_none() {
                        if let Some(i) = ["x", "y", "z"].iter().position(|&a| a == p.name) {
                            xyz[i] = value;
                        } else if colored && let Some(i) = colors.iter().position(|&a| a == p.name)
                        {
                            rgb[i] = value as u8;
                        } else {
                            ignored = true;
                        }
                    } else {
                        ignored = true;
                    }
                }
                if is_indices {
                    indices = Some(face_indices);
                }
            }
            if e.name == "vertex" {
                budget.vertex(&mut mesh, DVec3::from_array(xyz))?;
                if colored {
                    budget.reserve(&mut mesh.colors, 1)?;
                    mesh.colors.push(packed_color(rgb));
                }
            } else if e.name == "face" {
                mesh::polygon(
                    &mut mesh,
                    &indices.ok_or_else(|| bad(Key::MeshSyntax))?,
                    &mut budget,
                )?;
            }
        }
    }
    if data.encoding == Encoding::Ascii {
        while let Some(&b) = data.bytes.get(data.position) {
            if data.position.is_multiple_of(8192) {
                check_cancel(cancel)?;
            }
            if !b.is_ascii_whitespace() {
                return Err(bad(Key::MeshSyntax));
            }
            data.advance()?;
        }
    } else if data.position != data.bytes.len() {
        return Err(bad(Key::MeshSyntax));
    }
    let mut scene = mesh::scene(mesh);
    scene.warnings.push(Key::MeshUnitsWarning.into());
    if ignored {
        scene.warnings.push(Key::MeshAttributesWarning.into());
    }
    mesh::finish(scene, &mut budget)
}
