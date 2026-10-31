//! STL geometry only: coordinates are interpreted as millimeters; colors are not guessed.
use super::{ImportError, Limits, check_cancel};
use crate::{
    i18n::{Key, Message},
    scene::{Bounds, Instance, Mesh, Scene},
};
use glam::{DMat4, DVec3};
use std::{
    io::{BufRead, BufReader, Read, Seek, SeekFrom},
    sync::atomic::AtomicBool,
};

const LINE_BYTES: usize = 64 * 1024;
// Two line-buffer capacities, the read buffer, and scene/warning bookkeeping.
const OVERHEAD: usize = 2 * (LINE_BYTES + 1) + 8192 + 4096;

fn invalid(key: Key) -> ImportError {
    ImportError::Stl(key.into())
}

pub(super) fn binary_count(prefix: &[u8], length: u64) -> Option<u32> {
    let bytes: [u8; 4] = prefix.get(80..84)?.try_into().ok()?;
    let count = u32::from_le_bytes(bytes);
    (length == 84 + u64::from(count) * 50).then_some(count)
}

pub(super) fn read<R: Read + Seek>(
    mut input: R,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    check_cancel(cancel)?;
    let length = input.seek(SeekFrom::End(0))?;
    input.seek(SeekFrom::Start(0))?;
    if length > limits.archive_bytes {
        return Err(ImportError::Limit(Key::InputSize));
    }
    if limits.memory_bytes < OVERHEAD {
        return Err(ImportError::Limit(Key::GeometryMemory));
    }
    let mut header = [0; 84];
    let mut prefix = input.by_ref().take(84);
    let mut used = 0;
    while used < header.len() {
        check_cancel(cancel)?;
        let n = prefix.read(&mut header[used..])?;
        if n == 0 {
            break;
        }
        used += n;
    }
    // The exact binary record length takes precedence over a misleading "solid" header.
    let binary = binary_count(&header[..used], length);
    input.seek(SeekFrom::Start(0))?;
    let mut reader = BufReader::new(input.take(limits.archive_bytes.saturating_add(1)));
    let mut mesh = Builder {
        mesh: Mesh {
            vertices: vec![],
            triangles: vec![],
            painting: vec![],
            colors: vec![],
        },
        limits,
        cancel,
        seen: 0,
        skipped: 0,
        attributes: false,
    };
    if let Some(count) = binary {
        mesh.check_count(count as usize)?;
        let original_header = header;
        reader.read_exact(&mut header).map_err(binary_error)?;
        if header != original_header {
            return Err(invalid(Key::StlLength));
        }
        mesh.attributes = header[..80].windows(6).any(|w| w == b"COLOR=")
            || header[..80].windows(9).any(|w| w == b"MATERIAL=");
        for _ in 0..count {
            check_cancel(cancel)?;
            let mut facet = [0; 50];
            reader.read_exact(&mut facet).map_err(binary_error)?;
            let mut values = [0f64; 12];
            for (value, bytes) in values.iter_mut().zip(facet[..48].chunks_exact(4)) {
                *value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f64;
            }
            if !values[..3].iter().all(|v| v.is_finite()) {
                return Err(invalid(Key::StlNumber));
            }
            mesh.attributes |= facet[48..] != [0, 0];
            mesh.push([
                DVec3::from_slice(&values[3..6]),
                DVec3::from_slice(&values[6..9]),
                DVec3::from_slice(&values[9..12]),
            ])?;
        }
        if !reader.fill_buf()?.is_empty() {
            return Err(invalid(Key::StlLength));
        }
    } else {
        let mut text = Tokens {
            reader,
            line: Vec::new(),
            position: 0,
            line_number: 0,
            bytes: 0,
            limits,
            cancel,
        };
        let mut solids = 0;
        while let Some(token) = text.next()? {
            if token != "solid" {
                return Err(invalid(Key::StlFormat));
            }
            solids += 1;
            text.skip_name();
            loop {
                match text.next()? {
                    Some("endsolid") => {
                        text.skip_name();
                        break;
                    }
                    Some("facet") => {}
                    _ => return Err(text.syntax()),
                }
                text.expect("normal")?;
                text.vector()?; // Shading derives normals from the original vertex winding.
                text.expect("outer")?;
                text.expect("loop")?;
                let mut corners = [DVec3::ZERO; 3];
                for corner in &mut corners {
                    text.expect("vertex")?;
                    *corner = text.vector()?;
                }
                text.expect("endloop")?;
                text.expect("endfacet")?;
                mesh.push(corners)?;
            }
        }
        if solids == 0 {
            return Err(invalid(Key::StlFormat));
        }
    }
    mesh.finish()
}

fn binary_error(error: std::io::Error) -> ImportError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        invalid(Key::StlLength)
    } else {
        ImportError::Io(error)
    }
}

struct Builder<'a> {
    mesh: Mesh,
    limits: &'a Limits,
    cancel: &'a AtomicBool,
    seen: usize,
    skipped: usize,
    attributes: bool,
}
impl Builder<'_> {
    fn check_count(&self, count: usize) -> Result<(), ImportError> {
        if count > self.limits.triangles {
            return Err(ImportError::Limit(Key::TriangleCount));
        }
        Ok(())
    }
    fn push(&mut self, corners: [DVec3; 3]) -> Result<(), ImportError> {
        check_cancel(self.cancel)?;
        self.seen = self.seen.saturating_add(1);
        self.check_count(self.seen)?;
        if corners
            .iter()
            .any(|p| !p.is_finite() || p.abs().max_element() > 1e15)
        {
            return Err(invalid(Key::CoordinateRange));
        }
        let [a, b, c] = corners;
        if (b - a).cross(c - a).length_squared() == 0. {
            self.skipped += 1;
            return Ok(());
        }
        if self.mesh.triangles.len() >= self.limits.rendered_triangles {
            return Err(ImportError::Limit(Key::RenderedTriangles));
        }
        let first = u32::try_from(self.mesh.vertices.len())
            .ok()
            .filter(|&n| n <= u32::MAX - 2)
            .ok_or(ImportError::Limit(Key::VertexCount))?;
        // Reserve in bounded chunks. Charge both old and new allocations during growth.
        if self.mesh.triangles.len() == self.mesh.triangles.capacity()
            || self.mesh.vertices.len() + 3 > self.mesh.vertices.capacity()
        {
            let current = self
                .mesh
                .vertices
                .capacity()
                .saturating_mul(24)
                .saturating_add(self.mesh.triangles.capacity().saturating_mul(12));
            let affordable = self
                .limits
                .memory_bytes
                .saturating_sub(OVERHEAD)
                .saturating_sub(current)
                / 84;
            let target = self
                .mesh
                .triangles
                .len()
                .saturating_mul(2)
                .max(4096)
                .min(self.limits.triangles)
                .min(self.limits.rendered_triangles)
                .min(affordable);
            if target <= self.mesh.triangles.len() {
                return Err(ImportError::Limit(Key::GeometryMemory));
            }
            self.mesh
                .vertices
                .try_reserve_exact(target.saturating_mul(3) - self.mesh.vertices.len())
                .map_err(|_| ImportError::Limit(Key::VertexMemory))?;
            self.mesh
                .triangles
                .try_reserve_exact(target - self.mesh.triangles.len())
                .map_err(|_| ImportError::Limit(Key::TriangleMemory))?;
        }
        self.mesh.vertices.extend(corners);
        self.mesh.triangles.push([first, first + 1, first + 2]);
        Ok(())
    }
    fn finish(self) -> Result<Scene, ImportError> {
        check_cancel(self.cancel)?;
        if self.mesh.triangles.is_empty() {
            return Err(ImportError::Empty);
        }
        if self.limits.instances == 0 {
            return Err(ImportError::Limit(Key::InstanceCount));
        }
        let mut warnings = vec![Key::StlUnitsWarning.into()];
        if self.attributes {
            warnings.push(Key::StlAttributesWarning.into());
        }
        if self.skipped > 0 {
            warnings.push(Message::with(Key::DegenerateWarning, [self.skipped]));
        }
        let mut scene = Scene {
            meshes: vec![self.mesh],
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
            warnings,
            summaries: Default::default(),
            palette: vec![],
        };
        scene.compute_summaries(|| check_cancel(self.cancel))?;
        scene.bounds = scene.bounds_for(None).ok_or(ImportError::Empty)?;
        if !scene.bounds.size().is_finite()
            || scene.bounds.radius() <= 0.
            || scene.bounds.radius() > 1e15
        {
            return Err(invalid(Key::BoundsRange));
        }
        Ok(scene)
    }
}

struct Tokens<'a, R> {
    reader: R,
    line: Vec<u8>,
    position: usize,
    line_number: usize,
    bytes: u64,
    limits: &'a Limits,
    cancel: &'a AtomicBool,
}
impl<R: BufRead> Tokens<'_, R> {
    fn syntax(&self) -> ImportError {
        ImportError::Stl(Message::with(Key::StlSyntax, [self.line_number]))
    }
    fn next(&mut self) -> Result<Option<&str>, ImportError> {
        loop {
            check_cancel(self.cancel)?;
            while self.position < self.line.len() && self.line[self.position].is_ascii_whitespace()
            {
                self.position += 1;
            }
            if self.position < self.line.len() {
                let start = self.position;
                while self.position < self.line.len()
                    && !self.line[self.position].is_ascii_whitespace()
                {
                    self.position += 1;
                }
                return std::str::from_utf8(&self.line[start..self.position])
                    .map(Some)
                    .map_err(|_| self.syntax());
            }
            self.line.clear();
            self.position = 0;
            let read = self
                .reader
                .by_ref()
                .take((LINE_BYTES + 1) as u64)
                .read_until(b'\n', &mut self.line)?;
            self.bytes = self.bytes.saturating_add(read as u64);
            if self.bytes > self.limits.archive_bytes {
                return Err(ImportError::Limit(Key::InputSize));
            }
            if read > LINE_BYTES {
                return Err(ImportError::Limit(Key::StlLineSize));
            }
            if read == 0 {
                return Ok(None);
            }
            self.line_number += 1;
        }
    }
    fn skip_name(&mut self) {
        self.position = self.line.len();
    }
    fn expect(&mut self, expected: &str) -> Result<(), ImportError> {
        if self.next()? == Some(expected) {
            Ok(())
        } else {
            Err(self.syntax())
        }
    }
    fn vector(&mut self) -> Result<DVec3, ImportError> {
        let mut values = [0.; 3];
        for value in &mut values {
            *value = self
                .next()?
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .ok_or_else(|| invalid(Key::StlNumber))?;
        }
        Ok(DVec3::from_array(values))
    }
}
