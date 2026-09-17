use std::path::{Path, PathBuf};

use wasmi::{Engine, Linker, Module, Store};

use crate::error::MotionError;

/// Loaded `physics-lab` closed-form lesson (C-ABI).
pub struct WasmLesson {
    store: Store<()>,
    params_ptr: wasmi::Func,
    state_at: wasmi::Func,
    readouts_ptr: wasmi::Func,
    memory: wasmi::Memory,
    n_params: usize,
}

impl WasmLesson {
    pub fn load(path: &Path, n_params: usize) -> Result<Self, MotionError> {
        let bytes = std::fs::read(path).map_err(MotionError::Io)?;
        let engine = Engine::default();
        let module = Module::new(&engine, &bytes).map_err(|e| MotionError::Wasm(e.to_string()))?;
        let mut store = Store::new(&engine, ());
        let linker = Linker::new(&engine);
        let instance = linker
            .instantiate(&mut store, &module)
            .and_then(|i| i.start(&mut store))
            .map_err(|e| MotionError::Wasm(e.to_string()))?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| MotionError::MissingExport("memory".into()))?;
        let params_ptr = export_func(&mut store, &instance, "params_ptr")?;
        let state_at = export_func(&mut store, &instance, "state_at")?;
        let readouts_ptr = export_func(&mut store, &instance, "readouts_ptr")?;

        Ok(WasmLesson {
            store,
            params_ptr,
            state_at,
            readouts_ptr,
            memory,
            n_params,
        })
    }

    pub fn eval(&mut self, params: &[f64]) -> Result<[f64; 8], MotionError> {
        if params.len() != self.n_params {
            return Err(MotionError::ParamCount {
                expected: self.n_params,
                got: params.len(),
            });
        }
        let ptr = self
            .params_ptr
            .typed::<(), i32>(&self.store)
            .map_err(|e| MotionError::Wasm(e.to_string()))?
            .call(&mut self.store, ())
            .map_err(|e| MotionError::Wasm(e.to_string()))? as u32;

        write_f64_slice(&mut self.store, &self.memory, ptr, params)?;
        self.state_at
            .typed::<i32, i32>(&self.store)
            .map_err(|e| MotionError::Wasm(e.to_string()))?
            .call(&mut self.store, self.n_params as i32)
            .map_err(|e| MotionError::Wasm(e.to_string()))?;

        let rptr = self
            .readouts_ptr
            .typed::<(), i32>(&self.store)
            .map_err(|e| MotionError::Wasm(e.to_string()))?
            .call(&mut self.store, ())
            .map_err(|e| MotionError::Wasm(e.to_string()))? as u32;

        let mut out = [0.0; 8];
        for (i, slot) in out.iter_mut().enumerate() {
            *slot = read_f64(&self.store, &self.memory, rptr + i as u32 * 8)?;
        }
        Ok(out)
    }
}

fn export_func(
    store: &mut Store<()>,
    instance: &wasmi::Instance,
    name: &str,
) -> Result<wasmi::Func, MotionError> {
    instance
        .get_func(store, name)
        .ok_or_else(|| MotionError::MissingExport(name.into()))
}

fn write_f64_slice(
    store: &mut Store<()>,
    memory: &wasmi::Memory,
    ptr: u32,
    vals: &[f64],
) -> Result<(), MotionError> {
    let data = memory.data_mut(store);
    for (i, v) in vals.iter().enumerate() {
        let off = ptr as usize + i * 8;
        if off + 8 > data.len() {
            return Err(MotionError::MemoryBounds);
        }
        data[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
    Ok(())
}

fn read_f64(store: &Store<()>, memory: &wasmi::Memory, ptr: u32) -> Result<f64, MotionError> {
    let data = memory.data(store);
    let off = ptr as usize;
    if off + 8 > data.len() {
        return Err(MotionError::MemoryBounds);
    }
    Ok(f64::from_le_bytes(data[off..off + 8].try_into().unwrap()))
}

pub fn wasm_path_for_source(source: &str, root: &Path) -> Option<PathBuf> {
    let slug = source.strip_prefix("physics-lab:")?;
    let path = root.join("public/lessons").join(slug).join("lesson.wasm");
    path.exists().then_some(path)
}

pub fn physics_lab_root() -> PathBuf {
    if let Ok(p) = std::env::var("GUION_PHYSICS_LAB") {
        return PathBuf::from(p);
    }
    PathBuf::from("../physics-lab")
}
