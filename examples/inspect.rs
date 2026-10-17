//! Headless geometry inspection, useful for fixture verification and timing.
use std::{path::PathBuf, sync::atomic::AtomicBool, time::Instant};
use yo3mfs::import::{Limits, load};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: cargo run --release --example inspect -- model.3mf")?;
    let start = Instant::now();
    let scene = load(&path, &Limits::default(), &AtomicBool::new(false))?;
    let imported = start.elapsed();
    let prepared = yo3mfs::prepared::prepare(scene, &Limits::default(), &AtomicBool::new(false))?;
    println!(
        "{}",
        serde_json::json!({
            "instances": prepared.scene.instances.len(),
            "triangles": prepared.scene.triangle_count(None),
            "size_mm": prepared.scene.bounds.size().to_array(),
            "cpu_geometry_bytes": prepared.scene.estimated_bytes(),
            "gpu_buffer_bytes": prepared.gpu_bytes,
            "rendered_triangles": prepared.rendered_triangles,
            "painted_facets": prepared.scene.meshes.iter().map(|m| m.painting.len()).sum::<usize>(),
            "palette": prepared.scene.palette.iter().map(|c| c.map(yo3mfs::settings::hex)).collect::<Vec<_>>(),
            "instance_filaments": prepared.scene.instances.iter().map(|i| i.filament).collect::<Vec<_>>(),
            "import_seconds": imported.as_secs_f64(),
            "total_seconds": start.elapsed().as_secs_f64(),
            "warnings": prepared.scene.warnings,
        })
    );
    Ok(())
}
