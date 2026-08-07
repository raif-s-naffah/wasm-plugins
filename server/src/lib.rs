// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{config::config, error::WasmPluginError};
use std::{
    io::{Error, ErrorKind},
    path::PathBuf,
};
use wasmtime::{
    Config, Engine, ExternType, Instance, Linker, Memory, MemoryTypeBuilder, Module, Store,
    StoreLimits, StoreLimitsBuilder,
};
use wasmtime_wasi::{
    WasiCtxBuilder,
    p1::{self, WasiP1Ctx},
};

mod config;
pub mod error;

#[derive(Debug)]
struct Plugin {
    /// The WASM [Module] key (aka short name; e.g. `fx`) we use to construct
    /// a full file object name that is expected to be the WASM binary located
    /// in a known location.
    id: String,
    /// A single [Instance] of the WASM [Module] we use in our PoC.
    instance: Instance,
}

/// NOTE (rsn) 20260803 - i'm trying to control the use/waste of memory by
/// introducing a [Store] resource-limiter.
/// NOTE (rsn) 20260805 - this 1 Page / Plugin is NOT working :( Instantiating
/// a WASI P1 Module requires a minimum of 16 to 17 Pages of 64KB memory each
/// (~1MB) as discovered lately :(  fortunately i can amend the limiter at
/// runtime!  for now pre-set 2 alternatives
/// NOTE (rsn) 20260807 - changing the limiter at runtime works but does NOT
/// help finding out what is the available/used Store memory --at least i
/// couldnt find an API call that gives me those answers.  for now, stick w/
/// 1 limiter and a hard-wired limit that satisfies the requirements of the 2
/// plugins of interest.
struct MyState {
    state: WasiP1Ctx,
    limits: StoreLimits,
}

/// A glorified name for a structure grouping objects representing WASM core
/// concept elements.
///
/// Documentation of individual fields are lifted from
/// <https://docs.rs/wasmtime/46.0.1/wasmtime/index.html#core-concepts>.
///
/// For the purpose of this PoC, we limit the number of Plugins to two,
/// w/ one Instance each.
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
    linker: Linker<MyState>,

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
    store: Store<MyState>,

    /// collection of loaded + ready Plugin instances.
    plugins: Vec<Plugin>,
}

impl Default for PluginManager {
    fn default() -> Self {
        // NOTE (rsn) 20260807 - using either a custom Config to enable custom
        // page size when instantiating an Engine, or simply using the default,
        // and later instantiating a Store w/ a custom MemoryType using a 1KB
        // value as the page size yields the same error saying that ONLY 64KB or
        // 1-byte page sizes are allowed :(
        //
        // let engine = Engine::default();
        let engine = Engine::new(Config::new().wasm_custom_page_sizes(true))
            .expect("Failed configuring Engine w/ custom memory page size :(");
        let wasip1 = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_env()
            .build_p1();

        // NOTE (rsn) 20260806 - WASM binaries are created and packaged elsewhere.
        // what their minimum memory page requirement is unknown, and to my
        // knowledge is not part of WASI specs.  as a consequence setting a hard
        // limit for the store here, at creation time, may cause problems later.
        // For example i used to set a hard limit of 2 pages for the Store but
        // discovered that the 2 plugins i'm using require 16 (fx) and 17 pages
        // (xx) :(
        // NOTE (rsn) 20260807 - even though i found out one can modify a Store
        // "limiter" _after_ creating the Store, that doesn't help reducing min
        // memory requirements.  for now stick w/ a hard total of 33 pages each
        // being 64KB large.
        println!(
            "[DEBUG] Will limit Store memory to {} pages",
            config().store_max_page_count
        );
        let my_state = MyState {
            state: wasip1,
            limits: StoreLimitsBuilder::new()
                .instances(config().store_max_instance_count)
                .memory_size(config().store_max_page_count * 64 * 1024) // in bytes...
                .build(),
        };
        println!("[DEBUG] Store limits = {:?}", my_state.limits);
        let mut store = Store::new(&engine, my_state);
        store.limiter(|state| &mut state.limits);
        let mut linker = Linker::new(&engine);
        p1::add_to_linker_sync(&mut linker, |state: &mut MyState| &mut state.state)
            .expect("Failed adding synchronous version of WASI P1 functions to the Linker :(");
        // limit the store's memory... see
        // https://docs.rs/wasmtime/47.0.3/wasmtime/struct.MemoryTypeBuilder.html
        // IMPORTANT (rsn) 20260803 - page size can only be set to 1 byte (too
        // low), or 64 kilo-bytes (too high).  ideally page-size for our plugins
        // should be MAX_PAYLOAD_LENGTH (1024 bytes), w/ 1 page worth of bytes
        // per instance...
        let memory_type = MemoryTypeBuilder::new()
            // .page_size_log2(10) // 1024 bytes -  DOESN'T WORK :(
            .max(Some(config().store_max_page_count.try_into().expect(
                "Failed converting Store max memory page count: usize -> u64",
            )))
            .build()
            .expect("Failed configuring MemoryType :(");
        Memory::new(&mut store, memory_type)
            .expect("Failed creating + assigning Memory to Store :(");
        println!("[DEBUG] Created + assigned Memory to Store!");
        Self {
            engine,
            linker,
            store,
            plugins: Vec::new(),
        }
    }
}

impl PluginManager {
    /// Load, and compile a Plugin's [Module] given its ID, then instantiate
    /// and cache one single [Instance].
    pub fn load(&mut self, mid: &str) -> Result<(), WasmPluginError> {
        if self.plugins.iter().find(|p| p.id == mid).is_some() {
            println!("[DEBUG] Plugin '{}' is already loaded. Do nothing", mid);
            return Ok(());
        }

        // check if we still have room...
        if self.plugins.len() == config().store_max_instance_count {
            return Err(WasmPluginError::Runtime(format!(
                "Store is full. No room for Plugin '{}' :(",
                mid
            )));
        }

        println!("[DEBUG] About to load Plugin '{}'...", mid);
        let plugin_wasm_file = plugin_loc(mid)?;
        let module = Module::from_file(&self.engine, plugin_wasm_file)?;

        // ensure it exports 'memory' as a WASI P1 component should...
        let memory = module
            .get_export("memory")
            .ok_or(WasmPluginError::Runtime(format!(
                "'memory' export was NOT found in Plugin '{}' :(",
                mid
            )))?;

        match memory {
            ExternType::Memory(memory_type) => {
                if memory_type.is_shared() {
                    return Err(WasmPluginError::Runtime(format!(
                        "Plugin '{}' exports its memory as shared :(",
                        mid
                    )));
                }

                let mod_min_mem = memory_type.minimum();
                println!(
                    "[DEBUG] Module '{}' (export) memory requires at least {} page(s)",
                    mid, mod_min_mem
                );

                let store_max_page_count_u64: u64 = config()
                    .store_max_page_count
                    .try_into()
                    .expect("Failed converting Store max memory: usize -> u64");
                assert!(
                    mod_min_mem <= store_max_page_count_u64,
                    "Module '{}' minimum memory page count ({}) exceeds our Store upper limit ({}) :(",
                    mid,
                    mod_min_mem,
                    store_max_page_count_u64
                );
            }
            _ => {
                return Err(WasmPluginError::Runtime(format!(
                    "Plugin '{}' does NOT export its memory as expected :(",
                    mid
                )));
            }
        };

        println!("[DEBUG] About to instantiate Plugin '{}'...", mid);
        let instance = self.linker.instantiate(&mut self.store, &module)?;
        let plugin = Plugin {
            id: mid.to_owned(),
            instance,
        };
        self.plugins.push(plugin);

        println!("[DEBUG] Plugin '{}' is ready!", mid);
        Ok(())
    }

    /// Fetch a [Module] (aka Plugin) [Instance] given its ID. Raise
    /// [WasmPluginError] if it's not already loaded.
    fn get(&mut self, mid: &str /* Module ID */) -> Result<Instance, WasmPluginError> {
        if let Some(p) = self.plugins.iter().find(|p| p.id == mid) {
            Ok(p.instance)
        } else {
            Err(WasmPluginError::Runtime(format!(
                "Instance of Plugin '{}' was not found :(",
                mid
            )))
        }
    }

    /// Convenience method to call the _Interface_ single function of a [Module]
    /// (aka plugin) [Instance] w/ given parameters.
    pub fn call_hash(
        &mut self,
        mid: &str, /* Module (aka Plugin) ID */
        seed: u32,
        salt: u32,
        data: &str,
    ) -> Result<u32, WasmPluginError> {
        let data_bytes = data.as_bytes();
        let length = data_bytes.len();
        if length > config().max_payload_len {
            return Err(WasmPluginError::Runtime(format!(
                "Data length ({}) exceeds maximum allowed limit ({}) :(",
                length,
                config().max_payload_len
            )));
        }

        let instance = self.get(mid)?;
        let memory =
            instance
                .get_memory(&mut self.store, "memory")
                .ok_or(WasmPluginError::Runtime(format!(
                    "'memory' export was NOT found in plugin '{}' instance :(",
                    mid
                )))?;
        // NOTE (rsn) 20260804 - `wasmtime` documentation states..
        // "WebAssembly memories are made up of a whole number of pages, so the
        // byte size returned will always be a multiple of this memory's page
        // size. Note that different Wasm memories may have different page sizes.
        // You can get a memory's page size via the Memory::page_size method."
        let m_page_size = memory.page_size(&self.store);
        println!(
            "[DEBUG] memory of plugin '{}' has a page size of {} bytes (or {} KB)",
            mid,
            m_page_size,
            m_page_size / 1024
        );
        let m_data_size_bytes = memory.data_size(&self.store);
        let m_data_size_bytes_u64: u64 = m_data_size_bytes
            .try_into()
            .expect("Failed converting Memory data size: usize -> u64");
        let m_data_pages = m_data_size_bytes_u64
            .checked_div(m_page_size)
            .expect("Unexpected None when dividing memory-data-size by page-size");
        let m_data_pages_rem = m_data_size_bytes_u64
            .checked_rem(m_page_size)
            .expect("Unexpected None when finding memory-data-size % page-size");
        assert_eq!(m_data_pages_rem, 0);
        println!(
            "[DEBUG] linear memory of plugin '{}' instance is {} bytes (or {} pages)",
            mid, m_data_size_bytes, m_data_pages
        );
        // ensure it has enough space to accomodate provided data...
        if m_data_size_bytes < length {
            let msg = format!(
                "Linear memory ({} bytes) of plugin '{}' instance is too small :(",
                m_data_size_bytes, mid
            );
            return Err(WasmPluginError::Runtime(msg));
        }

        let func = instance.get_typed_func::<(
            u32, /* seed */
            u32, /* salt */
            u32, /* data offset */
            u32, /* data length */
        ) /* input args */,
        u32 /* result */>(&mut self.store, "hash")?;

        // copy data bytes to plugin linear memory...
        memory.data_mut(&mut self.store)[0..length].copy_from_slice(data_bytes);

        // invoke the function...
        let result = func.call(&mut self.store, (seed, salt, 0, length as u32))?;

        // scrub the used linear memory for peace of mind...
        memory.data_mut(&mut self.store)[0..length].fill(0x00);

        Ok(result)
    }
}

/// Return location of WASM files.
fn plugins_dir() -> PathBuf {
    PathBuf::from(format!("{}/plugins", config().server_dir))
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
