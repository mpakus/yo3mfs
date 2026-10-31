//! Validate arithmetic and sparse indices before calling the library's typed readers.
use super::*;
// gltf-json 1.4.1 narrows u64 references to u32. Validate before that lossy conversion.
pub(super) fn json_indices(
    value: &serde_json::Value,
    key: &str,
    cancel: &AtomicBool,
) -> Result<(), ImportError> {
    check_cancel(cancel)?;
    if matches!(key, "extras" | "extensions") {
        return Ok(());
    }
    match value {
        serde_json::Value::Object(object) => {
            for (name, child) in object {
                json_indices(
                    child,
                    if matches!(key, "attributes" | "targets") {
                        "index"
                    } else {
                        name
                    },
                    cancel,
                )?;
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                json_indices(child, key, cancel)?;
            }
        }
        serde_json::Value::Number(number)
            if matches!(
                key,
                "index"
                    | "bufferView"
                    | "buffer"
                    | "sampler"
                    | "node"
                    | "input"
                    | "output"
                    | "indices"
                    | "material"
                    | "camera"
                    | "children"
                    | "skin"
                    | "mesh"
                    | "nodes"
                    | "scene"
                    | "inverseBindMatrices"
                    | "skeleton"
                    | "joints"
                    | "source"
            ) && number.as_u64().is_none_or(|n| n > u32::MAX as u64) =>
        {
            return Err(bad(Key::GltfStructure));
        }
        _ => {}
    }
    Ok(())
}
fn span(
    offset: usize,
    count: usize,
    stride: usize,
    size: usize,
    length: usize,
) -> Result<(), ImportError> {
    if count == 0
        || stride < size
        || offset
            .checked_add(
                (count - 1)
                    .checked_mul(stride)
                    .ok_or_else(|| bad(Key::GltfStructure))?,
            )
            .and_then(|n| n.checked_add(size))
            .is_none_or(|end| end > length)
    {
        return Err(bad(Key::GltfStructure));
    }
    Ok(())
}
pub(super) fn accessors(
    doc: &Document,
    buffers: &[Cow<'_, [u8]>],
    budget: &Budget<'_>,
) -> Result<(), ImportError> {
    for view in doc.views() {
        check_cancel(budget.cancel)?;
        let buffer = buffers
            .get(view.buffer().index())
            .ok_or_else(|| bad(Key::GltfStructure))?;
        if view
            .offset()
            .checked_add(view.length())
            .is_none_or(|end| end > buffer.len())
        {
            return Err(bad(Key::GltfStructure));
        }
    }
    for a in doc.accessors() {
        check_cancel(budget.cancel)?;
        if a.count() == 0 || a.count() > budget.limits.triangles.saturating_mul(3) {
            return Err(ImportError::Limit(Key::VertexCount));
        }
        let size = a.size();
        let component = a.data_type().size();
        if !a.offset().is_multiple_of(component) {
            return Err(bad(Key::GltfStructure));
        }
        if let Some(view) = a.view() {
            let stride = view.stride().unwrap_or(size);
            if !stride.is_multiple_of(component)
                || view
                    .offset()
                    .checked_add(a.offset())
                    .is_none_or(|v| !v.is_multiple_of(component))
            {
                return Err(bad(Key::GltfStructure));
            }
            span(a.offset(), a.count(), stride, size, view.length())?;
        } else if a.sparse().is_none() {
            return Err(bad(Key::GltfStructure));
        }
        if let Some(sparse) = a.sparse() {
            if sparse.count() == 0 || sparse.count() > a.count() {
                return Err(bad(Key::GltfStructure));
            }
            let indices = sparse.indices();
            let iv = indices.view();
            let is = indices.index_type().size();
            let values = sparse.values();
            let vv = values.view();
            if iv.stride().is_some() || vv.stride().is_some() {
                return Err(bad(Key::GltfStructure));
            }
            span(indices.offset(), sparse.count(), is, is, iv.length())?;
            span(values.offset(), sparse.count(), size, size, vv.length())?;
            let start = iv.offset() + indices.offset();
            let data = buffers[iv.buffer().index()]
                .get(start..start + sparse.count() * is)
                .ok_or_else(|| bad(Key::GltfStructure))?;
            let mut last = None;
            for (n, b) in data.chunks_exact(is).enumerate() {
                if n % 4096 == 0 {
                    check_cancel(budget.cancel)?;
                }
                let i = match is {
                    1 => b[0] as usize,
                    2 => u16::from_le_bytes([b[0], b[1]]) as usize,
                    4 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize,
                    _ => return Err(bad(Key::GltfStructure)),
                };
                if i >= a.count() || last.is_some_and(|p| i <= p) {
                    return Err(bad(Key::GltfStructure));
                }
                last = Some(i);
            }
        }
    }
    // glTF requires a strict forest and prohibits combining matrix with TRS.
    for node in &doc.as_json().nodes {
        if node.matrix.is_some()
            && (node.translation.is_some() || node.rotation.is_some() || node.scale.is_some())
        {
            return Err(bad(Key::GltfStructure));
        }
    }
    Ok(())
}
