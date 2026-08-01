// SPDX-License-Identifier: GPL-3.0-or-later

use crate::error::WasmPluginError;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind},
    path::PathBuf,
};
use wasmtime::{Engine, ExternType, Instance, InstancePre, Linker, Module, Store};
use wasmtime_wasi::{
    WasiCtxBuilder,
    p1::{self, WasiP1Ctx},
};

pub mod error;

// where am i?
const SERVER_DIR: &str = env!("CARGO_MANIFEST_DIR");

/// Structure grouping objects representing Wasmtime core concept elements
/// to facilitate management of plugin implementations.
///
/// Documentation of individual fields are lifted from
/// <https://docs.rs/wasmtime/46.0.1/wasmtime/index.html#core-concepts>.
///
/// IMPORTANT (rsn) 20260713 - this is till a WIP.  need to sort out instance
/// life-cycle management.
pub struct PluginManager {
    /// [Engine] - a global compilation and runtime environment for _WebAssembly_.
    /// An [Engine] is an object that can be shared concurrently across threads
    /// and is created with a Config with many knobs for configuring behavior.
    /// Compiling or executing any _WebAssembly_ requires first configuring and
    /// creating an [Engine]. All _Modules_ and _Components_ belong to an [Engine],
    /// and typically there’s one [Engine] per process.
    engine: Engine,

    /// [Linker] - host functions are defined within a linker to provide them a
    /// string-based name which can be looked up when instantiating a WebAssembly
    /// module or component. Linkers are traditionally populated at startup and
    /// then reused for all future instantiations of all instances, assuming the
    /// set of host functions does not change over time. Host functions are
    /// `Fn(..) + Send + Sync`` and typically do not close over mutable state.
    /// Instead it’s recommended to store mutable state in the `T`` of `Store<T>`
    /// which is accessed through `Caller<'_, T>`` provided to host functions.
    linker: Linker<WasiP1Ctx>,

    /// [Store] - container for all information related to WebAssembly objects
    /// such as functions, instances, memories, etc. A `Store<T>` allows
    /// customization of the T to store arbitrary host data within a [Store].
    /// This host data can be accessed through host functions via the Caller
    /// function parameter in host-defined functions. A [Store] is required for
    /// all WebAssembly operations, such as calling a wasm function. The
    /// [Store] is passed in as a “context” to methods like Func::call.
    /// Dropping a [Store] will deallocate all memory associated with WebAssembly
    /// objects within the [Store]. A [Store] is cheap to create and destroy and
    /// does not GC objects such as unused instances internally, so it’s intended
    /// to be short-lived (or no longer than the instances it contains).
    store: Store<WasiP1Ctx>,

    /// NOTE (rsn) 20260713 - we cache the [InstancePre] so (a) we dont have to
    /// verify WASM multiple times, and (b) instantiate many instances faster.
    /// this maps module (a.k.a. plugin) ID to their [InstancePre].
    templates: HashMap<String, InstancePre<WasiP1Ctx>>,

    /// Mappings of IDs to Wasmtime Instances. A key here is a concatenation of
    /// a module (a.k.a. plugin) ID and an instance number/order spearated by
    /// '/'; e.g. 'fx/100'.
    instances: HashMap<String, Instance>,
}

impl Default for PluginManager {
    fn default() -> Self {
        let engine = Engine::default();

        let mut linker = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |s| s)
            .expect("Failed adding synchronous version of WASI P1 functions to the Linker :(");

        let wasip1 = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .inherit_stderr()
            .build_p1();
        let store = Store::new(&engine, wasip1);

        Self {
            engine,
            linker,
            store,
            templates: HashMap::with_capacity(Self::DEFAULT_MODULE_COUNT),
            instances: HashMap::with_capacity(Self::DEFAULT_INSTANT_COUNT),
        }
    }
}

impl PluginManager {
    const DEFAULT_MODULE_COUNT: usize = 4;
    const DEFAULT_INSTANT_COUNT: usize = 8;

    /// Load, compile, and cache a plugin's WASM file given its ID.
    pub fn load_plugin(&mut self, mid: &str) -> Result<(), WasmPluginError> {
        let plugin_wasm_file = plugin_loc(mid)?;
        let module = Module::from_file(&self.engine, plugin_wasm_file)?;

        // ensure it exports 'memory' as a WASI P1 component should...
        let memory = module
            .get_export("memory")
            .ok_or(WasmPluginError::Runtime(format!(
                "'memory' export was NOT found in plugin '{}'",
                mid
            )))?;
        match memory {
            ExternType::Memory(memory_type) => {
                // ensure it's NOT shared...
                if memory_type.is_shared() {
                    return Err(WasmPluginError::Runtime(format!(
                        "Plugin '{}' exports its memory as shared :(",
                        mid
                    )));
                }
            }
            _ => {
                return Err(WasmPluginError::Runtime(format!(
                    "Plugin '{}' does NOT export its memory as expected :(",
                    mid
                )));
            }
        };

        let instance_pre = self.linker.instantiate_pre(&module)?;
        self.templates.insert(mid.to_owned(), instance_pre);

        Ok(())
    }

    /// Return an [Instance] given it's full ID (plugin + instance IDs separated
    /// by a slash). Instantiate + cache it if not already known to us.
    /// Raise [WasmPluginError] if the plugin/module is NOT already loaded.
    pub fn get_instance(&mut self, id: &str) -> Result<Instance, WasmPluginError> {
        // NOTE (rsn) 2026013 - assume instance IDs are a concatenation of a
        // module ID, the '/' punctuation character, and an integer ID id-ing
        // the instance among siblings in the same module; for example: `fx/1`,
        // and `fx/2` are IDs of two `fx` instances.  For now, if the '/' is
        // missing we'll assume it's the first/only instance the caller's after.
        let parts: Vec<&str> = id.trim().split('/').collect();
        let (mid, iid) = if parts.len() == 1 {
            (parts[0], 1)
        } else {
            (parts[0], parts[1].parse::<u32>()?)
        };
        let full_id = format!("{}/{}", mid, iid);
        // do we already know about this?
        let it = match self.instances.get(&full_id) {
            Some(x) => *x,
            None => {
                // dont have it.  instantiate, cache + return...
                let inst_pre = self
                    .templates
                    .get(mid)
                    .ok_or(WasmPluginError::Runtime(format!("Unnown plugin '{}'", mid)))?;
                let instance = inst_pre.instantiate(&mut self.store)?;
                self.instances.insert(full_id, instance);
                instance
            }
        };
        Ok(it)
    }

    /// Convenience function to call the _Interface_ single function on a
    /// given plugin [Instance] given its full ID, using the given arguments.
    pub fn call_hash(
        &mut self,
        id: &str,
        seed: u32,
        salt: u32,
        data: &str,
    ) -> Result<u32, WasmPluginError> {
        let instance = self.get_instance(id)?;
        let linear_mem =
            instance
                .get_memory(&mut self.store, "memory")
                .ok_or(WasmPluginError::Runtime(format!(
                    "'memory' export was NOT found in instance {} :(",
                    id
                )))?;
        let func = instance.get_typed_func::<(
            u32, /* seed */
            u32, /* salt */
            u32, /* data offset */
            u32, /* data length */
        ) /* input args */,
        u32 /* result */>(&mut self.store, "hash")?;

        // copy data bytes to plugin linear memory...
        let data_bytes = data.as_bytes();
        let length = data_bytes.len();
        linear_mem.data_mut(&mut self.store)[0..length].copy_from_slice(data_bytes);

        // invoke the function...
        let result = func.call(&mut self.store, (seed, salt, 0, length as u32))?;

        // scrub the used linear memory for peace of mind...
        linear_mem.data_mut(&mut self.store)[0..length].fill(0x00);

        Ok(result)
    }
}

/// Return location of WASM files.
fn plugins_dir() -> PathBuf {
    PathBuf::from(format!("{}/plugins", SERVER_DIR))
}

/// Return file system location of a plugin's WASM file given its ID.
///
/// Assume WASM files are named, based on their ID, as `<ID>_plugin.wasm` and
/// are located in designated 'plugins' folder.
fn plugin_loc(id: &str) -> Result<String, WasmPluginError> {
    let mut it = plugins_dir();
    it.push(format!("{}_plugin.wasm", id));
    if !it.exists() {
        return Err(WasmPluginError::IO(Error::new(
            ErrorKind::NotFound,
            it.display().to_string(),
        )));
    }

    Ok(it.display().to_string())
}
