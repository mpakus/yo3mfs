//! Minimal safe host for the pinned occt-wasm C ABI. No WASI filesystem or network.
//! ABI/empty-sandbox behavior follows occt-wasm (MIT); see THIRD-PARTY-NOTICES.txt.
use super::{HOST_ALLOWANCE, invalid};
use crate::{
    i18n::{Key, Message},
    import::{ImportError, Limits, check_cancel},
    scene::{Bounds, Instance, Mesh, Scene},
};
use glam::{DMat4, DVec3};
use occt_wasm::{
    OcctKernel,
    wasmtime::{
        self, Caller, Engine, Instance as WasmInstance, Linker, Memory, Module, ResourceLimiter,
        Store, UpdateDeadline, WasmParams, WasmResults,
    },
};
use std::{
    io::Read,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const DEADLINE: Duration = Duration::from_secs(60);
const FACE_LIMIT: usize = 100_000;

fn failure(error: impl std::fmt::Display) -> ImportError {
    ImportError::Step(Message::with(Key::StepKernel, [error.to_string()]))
}

fn module() -> Result<&'static Module, ImportError> {
    static MODULE: OnceLock<Result<Module, Message>> = OnceLock::new();
    MODULE
        .get_or_init(|| {
            let mut config = OcctKernel::engine_config();
            // Only retain the compiled module in memory; never use the optional disk cache.
            config.cache(None);
            config.epoch_interruption(true);
            config.cranelift_opt_level(wasmtime::OptLevel::None);
            let engine = Engine::new(&config)
                .map_err(|e| Message::with(Key::StepKernel, [e.to_string()]))?;
            // A compatible replacement stays sandboxed and is compiled from WASM, never native code.
            if let Some(path) = std::env::var_os("YO3MFS_STEP_KERNEL") {
                let mut bytes = Vec::new();
                std::fs::File::open(path)
                    .and_then(|f| f.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes))
                    .map_err(|e| Message::with(Key::StepKernel, [e.to_string()]))?;
                if bytes.len() > 32 * 1024 * 1024 {
                    return Err(Key::StepKernelSize.into());
                }
                Module::new(&engine, bytes)
                    .map_err(|e| Message::with(Key::StepKernel, [e.to_string()]))
            } else {
                OcctKernel::compile(&engine)
                    .map_err(|e| Message::with(Key::StepKernel, [e.to_string()]))
            }
        })
        .as_ref()
        .map_err(|message| ImportError::Step(message.clone()))
}

#[derive(Debug, thiserror::Error)]
#[error("{:?}", Key::StepKernelTrap)]
struct HostFault;

struct Host {
    cancel: Arc<AtomicBool>,
    deadline: Instant,
    memory_cap: usize,
    memory_failed: bool,
}
impl ResourceLimiter for Host {
    fn memory_growing(
        &mut self,
        _: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > self.memory_cap || maximum.is_some_and(|max| desired > max) {
            self.memory_failed = true;
            // Returning false would let unchecked C++ allocation failures corrupt address zero.
            return Err(wasmtime::Error::new(HostFault));
        }
        Ok(true)
    }
    fn memory_grow_failed(&mut self, error: wasmtime::Error) -> wasmtime::Result<()> {
        self.memory_failed = true;
        Err(error)
    }
    fn table_growing(
        &mut self,
        _: usize,
        desired: usize,
        maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        Ok(desired <= 100_000 && maximum.is_none_or(|max| desired <= max))
    }
    fn instances(&self) -> usize {
        1
    }
    fn memories(&self) -> usize {
        1
    }
    fn tables(&self) -> usize {
        1
    }
}

struct Kernel {
    store: Store<Host>,
    instance: WasmInstance,
    memory: Memory,
}
impl Kernel {
    fn new(module: &Module, cap: usize, cancel: Arc<AtomicBool>) -> Result<Self, ImportError> {
        let mut store = Store::new(
            module.engine(),
            Host {
                cancel,
                deadline: Instant::now() + DEADLINE,
                memory_cap: cap,
                memory_failed: false,
            },
        );
        store.limiter(|host| host);
        store.set_epoch_deadline(1);
        store.epoch_deadline_callback(|context| {
            if context.data().cancel.load(Ordering::Relaxed)
                || Instant::now() >= context.data().deadline
            {
                Err(wasmtime::Error::new(HostFault))
            } else {
                Ok(UpdateDeadline::Continue(1))
            }
        });
        let mut linker = Linker::new(module.engine());
        define_imports(&mut linker).map_err(failure)?;
        linker
            .define_unknown_imports_as_traps(module)
            .map_err(failure)?;
        let instance = linker.instantiate(&mut store, module).map_err(|error| {
            if store.data().memory_failed {
                ImportError::Limit(Key::StepMemory)
            } else if error.is::<HostFault>() {
                invalid(Key::StepKernelTrap)
            } else {
                failure(error)
            }
        })?;
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| invalid(Key::StepIncomplete))?;
        let mut kernel = Self {
            store,
            instance,
            memory,
        };
        kernel.call::<_, ()>("_initialize", ())?;
        let status: i32 = kernel.call("occt_init", ())?;
        if status != 0 {
            return Err(invalid(Key::StepIncomplete));
        }
        Ok(kernel)
    }
    fn call<P: WasmParams, R: WasmResults>(
        &mut self,
        name: &str,
        args: P,
    ) -> Result<R, ImportError> {
        check_cancel(&self.store.data().cancel)?;
        let function = self
            .instance
            .get_typed_func::<P, R>(&mut self.store, name)
            .map_err(failure)?;
        function.call(&mut self.store, args).map_err(|e| {
            if self.store.data().cancel.load(Ordering::Relaxed) {
                ImportError::Cancelled
            } else if self.store.data().memory_failed {
                ImportError::Limit(Key::StepMemory)
            } else if Instant::now() >= self.store.data().deadline {
                invalid(Key::StepTimeout)
            } else if e.is::<HostFault>() {
                invalid(Key::StepKernelTrap)
            } else {
                failure(e)
            }
        })
    }
    fn check(&mut self) -> Result<(), ImportError> {
        let error: i32 = self.call("occt_has_error", ())?;
        if error != 0 {
            let pointer: u32 = self.call("occt_get_error", ())?;
            let length: u32 = self.call("occt_get_error_len", ())?;
            let message = self.bytes(pointer, (length as usize).min(4096))?;
            return Err(failure(String::from_utf8_lossy(message)));
        }
        Ok(())
    }
    fn write(&mut self, bytes: &[u8]) -> Result<u32, ImportError> {
        let pointer: u32 = self.call("occt_alloc", bytes.len() as u32)?;
        if pointer == 0 {
            return Err(ImportError::Limit(Key::StepMemory));
        }
        self.memory
            .write(&mut self.store, pointer as usize, bytes)
            .map_err(failure)?;
        Ok(pointer)
    }
    fn bytes(&self, pointer: u32, length: usize) -> Result<&[u8], ImportError> {
        let start = pointer as usize;
        let end = start
            .checked_add(length)
            .ok_or_else(|| invalid(Key::StepIncomplete))?;
        self.memory
            .data(&self.store)
            .get(start..end)
            .ok_or_else(|| invalid(Key::StepIncomplete))
    }
    fn integers(
        &self,
        pointer: u32,
        count: usize,
    ) -> Result<impl Iterator<Item = u32> + '_, ImportError> {
        Ok(self
            .bytes(
                pointer,
                count
                    .checked_mul(4)
                    .ok_or_else(|| invalid(Key::StepIncomplete))?,
            )?
            .chunks_exact(4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
    }
    fn bounds(&mut self, shape: u32) -> Result<Bounds, ImportError> {
        let status: i32 = self.call("occt_get_bounding_box", (shape, 0i32))?;
        self.check()?;
        if status < 0 {
            return Err(invalid(Key::StepIncomplete));
        }
        let mut coordinates = [0f64; 6];
        for (out, name) in coordinates.iter_mut().zip([
            "occt_get_bbox_xmin",
            "occt_get_bbox_ymin",
            "occt_get_bbox_zmin",
            "occt_get_bbox_xmax",
            "occt_get_bbox_ymax",
            "occt_get_bbox_zmax",
        ]) {
            *out = self.call(name, ())?;
        }
        if coordinates.iter().any(|v| !v.is_finite() || v.abs() > 1e15) {
            return Err(invalid(Key::CoordinateRange));
        }
        let bounds = Bounds {
            min: DVec3::from_slice(&coordinates[..3]),
            max: DVec3::from_slice(&coordinates[3..]),
        };
        if bounds.size().min_element() < 0. || bounds.radius() <= 0. {
            return Err(ImportError::Empty);
        }
        Ok(bounds)
    }
    fn face_hashes(&mut self, shape: u32) -> Result<Vec<u32>, ImportError> {
        let name = self.write(b"face")?;
        let count: i32 = self.call(
            "occt_sub_shape_hashes",
            (shape, name as i32, 4i32, i32::MAX),
        )?;
        self.check()?;
        self.call::<_, ()>("occt_free", name)?;
        if count <= 0 {
            return Err(ImportError::Empty);
        }
        if count as usize > FACE_LIMIT {
            return Err(ImportError::Limit(Key::StepFaces));
        }
        let pointer: u32 = self.call("occt_get_vec_i32_result", ())?;
        let mut hashes: Vec<u32> = self.integers(pointer, count as usize)?.collect();
        hashes.sort_unstable();
        if hashes.windows(2).any(|w| w[0] == w[1]) {
            // Reject hash collisions: they must not conceal an omitted face.
            return Err(invalid(Key::StepIncomplete));
        }
        Ok(hashes)
    }
    fn mesh(
        &mut self,
        shape: u32,
        offset: DVec3,
        tolerance: f64,
        input_capacity: usize,
        limits: &Limits,
    ) -> Result<Scene, ImportError> {
        let expected_faces = self.face_hashes(shape)?;
        let status: i32 = self.call("occt_tessellate", (shape, tolerance, 0.3f64))?;
        self.check()?;
        if status < 0 {
            return Err(invalid(Key::StepIncomplete));
        }
        let positions: u32 = self.call("occt_get_mesh_positions", ())?;
        let coordinates: u32 = self.call("occt_get_mesh_positions_len", ())?;
        let indices: u32 = self.call("occt_get_mesh_indices", ())?;
        let index_count: u32 = self.call("occt_get_mesh_indices_len", ())?;
        let groups: u32 = self.call("occt_get_mesh_face_groups", ())?;
        let group_count: u32 = self.call("occt_get_mesh_face_groups_len", ())?;
        if !coordinates.is_multiple_of(3)
            || !index_count.is_multiple_of(3)
            || !group_count.is_multiple_of(3)
            || index_count == 0
        {
            return Err(invalid(Key::StepIncomplete));
        }
        let triangles = index_count as usize / 3;
        if triangles > limits.triangles {
            return Err(ImportError::Limit(Key::TriangleCount));
        }
        if triangles > limits.rendered_triangles {
            return Err(ImportError::Limit(Key::RenderedTriangles));
        }
        let host_bytes = (coordinates as usize)
            .saturating_mul(8)
            .saturating_add(index_count as usize * 4);
        let overhead =
            HOST_ALLOWANCE + input_capacity + self.store.data().memory_cap + FACE_LIMIT * 8;
        if overhead.saturating_add(host_bytes) > limits.memory_bytes {
            return Err(ImportError::Limit(Key::StepMemory));
        }
        let mut seen = vec![false; expected_faces.len()];
        let mut end = 0u32;
        let mut values = self.integers(groups, group_count as usize)?;
        while let (Some(start), Some(length), Some(hash)) =
            (values.next(), values.next(), values.next())
        {
            check_cancel(&self.store.data().cancel)?;
            if start != end || length == 0 || !length.is_multiple_of(3) {
                return Err(invalid(Key::StepIncomplete));
            }
            end = end
                .checked_add(length)
                .ok_or_else(|| invalid(Key::StepIncomplete))?;
            let face = expected_faces
                .binary_search(&hash)
                .map_err(|_| invalid(Key::StepIncomplete))?;
            seen[face] = true;
        }
        if end != index_count || seen.contains(&false) {
            return Err(invalid(Key::StepIncomplete));
        }
        drop(values);
        let mut mesh = Mesh {
            colors: vec![],
            vertices: Vec::new(),
            triangles: Vec::new(),
            painting: Vec::new(),
        };
        mesh.vertices
            .try_reserve_exact(coordinates as usize / 3)
            .map_err(|_| ImportError::Limit(Key::StepMemory))?;
        mesh.triangles
            .try_reserve_exact(triangles)
            .map_err(|_| ImportError::Limit(Key::StepMemory))?;
        for (i, bytes) in self
            .bytes(positions, coordinates as usize * 4)?
            .chunks_exact(12)
            .enumerate()
        {
            if i % 4096 == 0 {
                check_cancel(&self.store.data().cancel)?;
            }
            let mut xyz = [0.; 3];
            for (v, b) in xyz.iter_mut().zip(bytes.chunks_exact(4)) {
                *v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64;
            }
            let point = DVec3::from_array(xyz);
            if !point.is_finite() || (point + offset).abs().max_element() > 1e15 {
                return Err(invalid(Key::CoordinateRange));
            }
            mesh.vertices.push(point);
        }
        let mut values = self.integers(indices, index_count as usize)?;
        while let (Some(a), Some(b), Some(c)) = (values.next(), values.next(), values.next()) {
            if mesh.triangles.len().is_multiple_of(4096) {
                check_cancel(&self.store.data().cancel)?;
            }
            if [a, b, c].iter().any(|i| *i as usize >= mesh.vertices.len()) {
                return Err(invalid(Key::BadIndex));
            }
            let [va, vb, vc] = [a, b, c].map(|i| mesh.vertices[i as usize]);
            if (vb - va).cross(vc - va).length_squared() == 0. {
                return Err(invalid(Key::StepIncomplete));
            }
            mesh.triangles.push([a, b, c]);
        }
        let mut scene = Scene {
            meshes: vec![mesh],
            instances: vec![Instance {
                mesh: 0,
                transform: DMat4::from_translation(offset),
                name: String::new(),
                plate: None,
                filament: 0,
            }],
            bounds: Bounds {
                min: DVec3::ZERO,
                max: DVec3::ZERO,
            },
            warnings: vec![
                Key::StepPreviewWarning.into(),
                Key::StepConversionWarning.into(),
            ],
            summaries: Default::default(),
            palette: Vec::new(),
        };
        scene.compute_summaries(|| check_cancel(&self.store.data().cancel))?;
        scene.bounds = scene.bounds_for(None).ok_or(ImportError::Empty)?;
        Ok(scene)
    }
}

pub(super) fn convert(
    data: &[u8],
    capacity: usize,
    cap: usize,
    limits: &Limits,
    cancel: &AtomicBool,
) -> Result<Scene, ImportError> {
    let module = module()?; // Compile the fixed kernel once, off the UI thread, before the conversion deadline.
    check_cancel(cancel)?;
    let stopped = AtomicBool::new(false);
    let interrupted = Arc::new(AtomicBool::new(false));
    struct StopTimer<'a>(&'a AtomicBool);
    impl Drop for StopTimer<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Relaxed);
        }
    }
    std::thread::scope(|scope| {
        // A scoped timer always exits on success, error, or cancellation; no permanent worker queue.
        scope.spawn(|| {
            while !stopped.load(Ordering::Relaxed) {
                if cancel.load(Ordering::Relaxed) {
                    interrupted.store(true, Ordering::Relaxed);
                }
                module.engine().increment_epoch();
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let _stop = StopTimer(&stopped);
        let result = (|| {
            let mut kernel = Kernel::new(module, cap, interrupted.clone())?;
            let pointer = kernel.write(data)?;
            let shape: u32 =
                kernel.call("occt_import_step", (pointer as i32, data.len() as i32))?;
            kernel.check()?;
            kernel.call::<_, ()>("occt_free", pointer)?;
            if shape == 0 {
                return Err(ImportError::Empty);
            }
            let bounds = kernel.bounds(shape)?;
            let center = bounds.center();
            // OCCT operates on doubles; move near the origin BEFORE its f32 mesh extraction.
            let centered: u32 =
                kernel.call("occt_translate", (shape, -center.x, -center.y, -center.z))?;
            kernel.check()?;
            if centered == 0 {
                return Err(invalid(Key::StepIncomplete));
            }
            // Absolute millimeters, bounded relative to size for very small/large models.
            let tolerance = (bounds.size().max_element() * 0.0001).clamp(0.001, 0.1);
            kernel.mesh(centered, center, tolerance, capacity, limits)
        })();
        check_cancel(cancel)?;
        result
    })
}

fn memory(caller: &mut Caller<'_, Host>) -> wasmtime::Result<Memory> {
    caller
        .get_export("memory")
        .and_then(wasmtime::Extern::into_memory)
        .ok_or_else(|| wasmtime::Error::new(HostFault))
}
fn define_imports(linker: &mut Linker<Host>) -> wasmtime::Result<()> {
    const WASI: &str = "wasi_snapshot_preview1";
    linker.func_wrap(WASI, "proc_exit", |_code: i32| -> wasmtime::Result<()> {
        Err(wasmtime::Error::new(HostFault))
    })?;
    linker.func_wrap(
        WASI,
        "fd_write",
        |mut caller: Caller<'_, Host>,
         fd: i32,
         iovs: u32,
         count: u32,
         written: u32|
         -> wasmtime::Result<i32> {
            if fd != 1 && fd != 2 {
                return Ok(8);
            }
            if count > 65536 {
                return Err(wasmtime::Error::new(HostFault));
            }
            let memory = memory(&mut caller)?;
            let mut total = 0u32;
            for i in 0..count {
                let address = (iovs as usize)
                    .checked_add(i as usize * 8 + 4)
                    .ok_or_else(|| wasmtime::Error::new(HostFault))?;
                let mut size = [0; 4];
                memory.read(&caller, address, &mut size)?;
                total = total.saturating_add(u32::from_le_bytes(size));
            }
            memory.write(&mut caller, written as usize, &total.to_le_bytes())?;
            Ok(0)
        },
    )?;
    linker.func_wrap(WASI, "fd_read", |_: i32, _: i32, _: i32, _: i32| 8i32)?;
    linker.func_wrap(WASI, "fd_seek", |_: i32, _: i64, _: i32, _: i32| 8i32)?;
    linker.func_wrap(WASI, "fd_close", |_: i32| 8i32)?;
    linker.func_wrap(
        WASI,
        "environ_sizes_get",
        |mut caller: Caller<'_, Host>, count: u32, size: u32| -> wasmtime::Result<i32> {
            let memory = memory(&mut caller)?;
            memory.write(&mut caller, count as usize, &[0; 4])?;
            memory.write(&mut caller, size as usize, &[0; 4])?;
            Ok(0)
        },
    )?;
    linker.func_wrap(WASI, "environ_get", |_: i32, _: i32| 0i32)?;
    linker.func_wrap(
        WASI,
        "clock_time_get",
        |mut caller: Caller<'_, Host>, _: i32, _: i64, time: u32| -> wasmtime::Result<i32> {
            let memory = memory(&mut caller)?;
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(wasmtime::Error::from)?
                .as_nanos();
            let nanos = u64::try_from(nanos).map_err(wasmtime::Error::from)?;
            memory.write(&mut caller, time as usize, &nanos.to_le_bytes())?;
            Ok(0)
        },
    )?;
    linker.func_wrap("env", "emscripten_notify_memory_growth", |_: i32| {})?;
    linker.func_wrap(
        "env",
        "emscripten_get_callstack",
        |_: i32, _: i32, _: i32| 0i32,
    )?;
    linker.func_wrap(
        "env",
        "emscripten_get_preloaded_image_data",
        |_: i32, _: i32, _: i32| 0i32,
    )?;
    linker.func_wrap("env", "OSD_MemInfo_getModuleHeapLength", || 0f64)?;
    linker.func_wrap("env", "_emscripten_lookup_name", |_: i32| 0i32)?;
    linker.func_wrap(
        "env",
        "__syscall_faccessat",
        |_: i32, _: i32, _: i32, _: i32| -52i32,
    )?;
    linker.func_wrap("env", "__syscall_chmod", |_: i32, _: i32| -52i32)?;
    linker.func_wrap("env", "__syscall_getdents64", |_: i32, _: i32, _: i32| {
        -52i32
    })?;
    linker.func_wrap("env", "__syscall_unlinkat", |_: i32, _: i32, _: i32| -52i32)?;
    linker.func_wrap("env", "__syscall_rmdir", |_: i32| -52i32)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_kernel() -> Kernel {
        let mut config = OcctKernel::engine_config();
        config.epoch_interruption(true);
        let engine = Engine::new(&config).unwrap();
        let module = Module::new(
            &engine,
            r#"(module
            (memory (export "memory") 1 4)
            (func (export "_initialize"))
            (func (export "occt_init") (result i32) i32.const 0)
            (func (export "grow") (result i32) i32.const 1 memory.grow)
            (func (export "spin") (loop $again br $again))
            (func (export "trap") unreachable)
        )"#,
        )
        .unwrap();
        Kernel::new(&module, 65536, Arc::new(AtomicBool::new(false))).unwrap()
    }

    #[test]
    fn sandbox_growth_traps_instead_of_returning_null_and_traps_are_errors() {
        let mut kernel = tiny_kernel();
        assert!(matches!(
            kernel.call::<_, i32>("grow", ()),
            Err(ImportError::Limit(Key::StepMemory))
        ));
        let mut kernel = tiny_kernel();
        assert!(matches!(
            kernel.call::<_, ()>("trap", ()),
            Err(ImportError::Step(_))
        ));
    }

    #[test]
    fn epoch_cancellation_interrupts_a_running_kernel() {
        let mut kernel = tiny_kernel();
        let cancel = kernel.store.data().cancel.clone();
        let engine = kernel.store.engine().clone();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(30));
                cancel.store(true, Ordering::Relaxed);
                engine.increment_epoch();
            });
            assert!(matches!(
                kernel.call::<_, ()>("spin", ()),
                Err(ImportError::Cancelled)
            ));
        });
    }

    #[test]
    fn epoch_deadline_interrupts_a_running_kernel() {
        let mut kernel = tiny_kernel();
        kernel.store.data_mut().deadline = Instant::now() + Duration::from_millis(10);
        let engine = kernel.store.engine().clone();
        std::thread::scope(|scope| {
            scope.spawn(|| {
                std::thread::sleep(Duration::from_millis(30));
                engine.increment_epoch();
            });
            assert!(
                matches!(kernel.call::<_, ()>("spin", ()), Err(ImportError::Step(message)) if message.key == Key::StepTimeout)
            );
        });
    }
}
