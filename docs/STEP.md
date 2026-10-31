# STEP preview in YO3MFS 1.5.0

Open `.step` or `.stp` through File → Open, drag-and-drop, a startup path, or the headless `inspect` example. The viewer reads local files without modifying them. STEP uses the existing camera, background, material selector, loading/error state, and English/Russian interface.

## Supported profile

The importer accepts uncompressed, classic ISO 10303-21 text with one HEADER and one DATA section. Open CASCADE converts embedded B-rep geometry, including curved surfaces and assembly placements, into a neutral triangle mesh. Source units are converted to millimeters by the CAD engine. Assembly occurrences retain their world placements but are flattened into one displayed mesh/instance; the inspector's triangle count is the tessellated preview count. This is not an editable CAD document or a hierarchy browser.

Colors, product names, PMI, validation properties, layers, external component files, compressed STEP, STEP XML, and multi-DATA-section exchanges are outside this profile. Both persistent import notes explain the preview and conversion limitations. Bambu painting remains specific to 3MF; STL and STEP display neutral material.

The pinned OCCT importer applies its default topology normalization/healing. Its facade checks the read result and requires geometry, but does not expose per-root transfer diagnostics. Unsupported entities or external components can therefore be omitted without an exact omission count. The persistent warning explicitly asks users to compare the preview with their CAD source. Do not describe this as lossless or complete STEP certification. No source geometry is rewritten, exported, or sent over the network.

Tessellation uses absolute deflection `clamp(max_dimension_mm × 0.0001, 0.001, 0.1)` and angular deflection 0.3 radians. Bounds and dimensions describe the resulting mesh, so curved extrema can differ from exact CAD bounds by tessellation tolerance. The whole shape is translated near its double-precision bounding-box center before OCCT emits float vertices, then the original offset is restored as a double-precision instance transform. Very large scenes containing tiny separated details remain limited by float tessellation precision; collapsed facets are rejected.

The host explicitly disables Wasmtime's optional persistent disk cache. Only the fixed compiled kernel is retained in process memory; model data is never cached.

## Import boundaries

- Read at most 64 MiB, or the caller's smaller shared input limit, with cancellation between read batches. No temporary model files are created.
- Check Part 21 framing, section order, balanced parentheses/quotes/comments, and instance-record syntax before conversion. Schema interpretation and reference resolution remain OCCT's responsibility; the preflight is not a complete STEP parser.
- Run the CAD kernel in a Wasmtime store with a maximum 512 MiB of linear memory, reduced for a smaller shared budget. The kernel starts with 128 MiB. Refused memory growth traps the operation rather than allowing unchecked C++ allocation failures to continue.
- Account for host input capacity, WASM memory, face bookkeeping, and output geometry. Reserve 256 MiB for host/runtime bookkeeping. The shared 2 GiB budget is accounting, not an OS RSS ceiling; fixed-kernel compilation, cached machine code, the UI, and driver/runtime overhead require separate measurement.
- Check at most 100,000 unique faces, 5 million output triangles, 10 million rendered triangles, finite coordinates, valid indices, and the existing scene/GPU limits. Custom lower limits apply. Degenerate STEP tessellation is rejected rather than silently repaired.
- Compare unique imported face hashes with tessellation groups, including contiguous index ranges and nonempty triangles. Reject hash collisions and missing faces. This verifies faces that OCCT imported; it cannot prove that every source root transferred.
- A scoped 20 ms epoch timer interrupts running WASM after cancellation or a 60-second conversion deadline. The timer exits on every result. The first STEP load compiles the fixed kernel once on the import worker; compilation precedes the deadline and is not interruptible. A canceled request is checked immediately afterward and can never publish.
- The host supplies no filesystem, network, environment, or external document access. Unknown host calls trap. Output from kernel stdout/stderr is discarded; bounded technical kernel errors are returned through localized typed messages.

Only one import worker and one replaceable pending request exist. Failure, cancellation, and outdated completion retain the previous successfully loaded scene. The kernel and its per-document memory are dropped before GPU preparation; compiled code is reused for later STEP loads in the same process.

## Dependency and source provenance

The pinned crate is [occt-wasm 5.0.1](https://crates.io/crates/occt-wasm/5.0.1), running [Wasmtime 48.0.8](https://github.com/bytecodealliance/wasmtime/tree/v48.0.8). Its Rust facade/host is MIT or Apache-2.0; the embedded Open CASCADE kernel retains LGPL-2.1 with the OCCT exception. YO3MFS uses the MIT terms for adapted host behavior. Project code remains safe Rust; WebAssembly does not add a WebView or JavaScript frontend.

Pinned source revisions:

- [occt-wasm facade and build tools](https://github.com/andymai/occt-wasm/tree/d01b8c9e61537c5c09feeb34592d75220eaad263).
- [OCCT fork](https://github.com/andymai/OCCT/tree/c7f60ffe85f28c67358d1771454044f0521f2fdd), selected by that revision's submodule.
- [STEP import implementation](https://github.com/andymai/occt-wasm/blob/d01b8c9e61537c5c09feeb34592d75220eaad263/facade/generated/kernel.cpp#L3536).
- [Tessellation and face groups](https://github.com/andymai/occt-wasm/blob/d01b8c9e61537c5c09feeb34592d75220eaad263/facade/src/kernel.cpp#L380).

Packages include [THIRD-PARTY-NOTICES.txt](../THIRD-PARTY-NOTICES.txt) and the terms from [assets/licenses](../assets/licenses). This STEP-specific review does not select the application's license or complete a whole-dependency distribution audit. Before distribution, preserve the notices and provide corresponding library source/build materials in accordance with the LGPL; do not treat a link alone as proof that every obligation is satisfied.

A compatible raw WASM replacement can be selected before launching:

```bash
YO3MFS_STEP_KERNEL=/absolute/path/to/occt-wasm-wasi.wasm ./yo3mfs model.step
```

On Windows, set `YO3MFS_STEP_KERNEL` in the launching process environment. On macOS, launch `yo3mfs.app/Contents/MacOS/yo3mfs` from that environment. The maximum replacement size is 32 MiB; it must expose the pinned ABI. It is compiled from WebAssembly, not loaded as serialized native machine code, and runs under the same sandbox limits. Restart to change kernels. Reverse engineering to debug modifications to the LGPL library is not restricted by YO3MFS.

To rebuild the library, install CMake and Ninja, activate Emscripten **5.0.3**, and follow the pinned source toolchain (Rust 1.95, separate from YO3MFS's 1.97.1):

```bash
git clone https://github.com/andymai/occt-wasm.git
cd occt-wasm
git checkout d01b8c9e61537c5c09feeb34592d75220eaad263
git submodule update --init --recursive
bash scripts/fetch-rapidjson.sh
cargo xtask build-occt
cargo xtask build-wasi --release
```

The raw replacement is `dist/occt-wasm-wasi.wasm`; the build also produces the Brotli-compressed crate asset. Rebuild instructions were inspected against the pinned source; a kernel rebuild has not been performed locally.

## Packaging and evidence

Normal YO3MFS builds use the crate's embedded WASM and need no OCCT/CMake/Emscripten installation. The runtime supports the project's four target architectures, but the new dependency must pass each native CI job and separate manual GUI acceptance. Older build results do not validate version 1.5.0.

Wasmtime 48.0.8 uses executable memory without `MAP_JIT` on macOS. Hardened builds therefore use the specific `com.apple.security.cs.allow-unsigned-executable-memory` entitlement in [entitlements.plist](../assets/macos/entitlements.plist); `allow-jit` alone is insufficient for this version. No debugger, library-validation, or application-sandbox exceptions are added. See [upstream discussion](https://github.com/bytecodealliance/wasmtime/issues/11989), [signing instructions](MACOS-SIGNING.md), and the local evidence in [VALIDATION.md](VALIDATION.md).

Synthetic fixtures are generated with [generate_step_fixtures.rs](../examples/generate_step_fixtures.rs), using the pinned OCCT exporter, and recorded with SHA-256 in [manifest.json](../tests/fixtures/manifest.json). Tests compare coordinates, winding/volume, dimensions, units, curved geometry, repeated placements, and distant geometry numerically. These files are not evidence for all AP203/AP214/AP242 exporters. A broader CAD-exporter corpus and native acceptance remain release validation work.
