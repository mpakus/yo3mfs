//! Reproduce synthetic STEP fixtures with the pinned occt-wasm kernel.
//! This is a development utility, not an application export feature.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = occt_wasm::OcctKernel::new()?;
    let block = kernel.make_box(10., 20., 30.)?;
    let block = kernel.translate(block, -2., -3., -4.)?;
    let cylinder = kernel.make_cylinder(5., 20.)?;
    let second = kernel.translate(block, 40., 0., 0.)?;
    let assembly = kernel.make_compound(&[block, second])?;
    let distant = kernel.translate(block, 1_000_000_000., 0., 0.)?;
    for (name, shape) in [
        ("sample-box.step", block),
        ("sample-cylinder.stp", cylinder),
        ("sample-assembly.step", assembly),
        ("sample-distant.step", distant),
    ] {
        std::fs::write(
            std::path::Path::new("tests/fixtures").join(name),
            kernel.export_step(shape)?,
        )?;
    }
    Ok(())
}
