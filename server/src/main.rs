// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    env,
    io::{Error, ErrorKind},
    path::PathBuf,
};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::{WasiCtxBuilder, p1};

// where am i?
const SERVER_DIR: &str = env!("CARGO_MANIFEST_DIR");
// the 2 plugin IDs...
const XX_PLUGIN_ID: &str = "xx";
const FX_PLUGIN_ID: &str = "fx";

/// Return file system location of a plugin given its ID.
///
/// Assume WASM files are named, based on their ID, as `<ID>_plugin.wasm` and
/// are located in designated 'plugins' path.
fn plugin_loc(plugins_dir: &PathBuf, id: &str) -> Result<String, Error> {
    let mut it = PathBuf::from(plugins_dir);
    it.push(format!("{}_plugin.wasm", id));
    if !it.exists() {
        eprintln!("Missing '{}_plugin' WASM... :(", id);
        return Err(Error::new(ErrorKind::NotFound, it.display().to_string()));
    }

    Ok(it.display().to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server_plugins = PathBuf::from(format!("{}/plugins", SERVER_DIR));
    println!("server_plugins={}", server_plugins.display());

    // load both WASM plugins...
    let xx_wasm = plugin_loc(&server_plugins, XX_PLUGIN_ID)?;
    let fx_wasm = plugin_loc(&server_plugins, FX_PLUGIN_ID)?;

    // setup WASM runtime for using the 2 plugins...
    // see https://docs.wasmtime.dev/examples-wasip1.html
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    p1::add_to_linker_sync(&mut linker, |s| s)?;

    // create a Context and use it to instantiate a Store.  all plugins
    // in this store share this same context.
    let wasip1 = WasiCtxBuilder::new()
        .inherit_stdio()
        .inherit_env()
        .inherit_stderr()
        .build_p1();
    let mut store = Store::new(&engine, wasip1);

    // instantiate plugin modules...
    let xx_module = Module::from_file(&engine, xx_wasm)?;
    let fx_module = Module::from_file(&engine, fx_wasm)?;

    // create a `pre_` instance to allow for multiple intantiations of the
    // corresponding plugins.
    // 
    // see https://docs.rs/wasmtime/45.0.2/wasmtime/struct.Linker.html#method.instantiate_pre)
    // for more details...
    let pre_xx = linker.instantiate_pre(&xx_module)?;
    let pre_fx = linker.instantiate_pre(&fx_module)?;

    // get access to the plugin's linear memory to pass our data bytes...
    let xx_instance = pre_xx.instantiate(&mut store)?;
    let xx_linear_mem = xx_instance
        .get_memory(&mut store, "memory")
        .ok_or("'memory' export was not found in xx-plugin :(")?;
    // ... as well as to the plugin's C wrapper which will convert
    // the data byte array (pointer + length) to a Rust friendly slice...
    let fn_xx_digest = xx_instance.get_typed_func::<(
        u32, /* seed */
        u32, /* salt */
        u32, /* data byte-array ptr */
        u32, /* data byte-array length */
    ) /* input args */,
    u32 /* result */>(&mut store, "hash")?;

    // ditto for the 2nd plugin...
    let fx_instance = pre_fx.instantiate(&mut store)?;
    let fx_linear_mem = fx_instance
        .get_memory(&mut store, "memory")
        .ok_or("'memory' export was not found in fx-plugin :(")?;
    let fn_fx_digest =
        fx_instance.get_typed_func::<(u32, u32, u32, u32), u32>(&mut store, "hash")?;

    // ready...
    let seed: u32 = 42;
    let salt: u32 = 1234;
    let data = "noone@nowhere.net:secret-password";
    let data_bytes = data.as_bytes();
    let z_len = data_bytes.len();

    // copy data bytes to both plugins' linear memory areas...
    xx_linear_mem.data_mut(&mut store)[0..z_len].copy_from_slice(data_bytes);
    fx_linear_mem.data_mut(&mut store)[0..z_len].copy_from_slice(data_bytes);

    // ...call the interface/trait function for each plugin...
    let res_xx = fn_xx_digest.call(&mut store, (seed, salt, 0, z_len as u32))?;
    let res_fx = fn_fx_digest.call(&mut store, (seed, salt, 0, z_len as u32))?;
    println!("params: {}, {}, {}", seed, salt, data);
    println!("Result (xx): {}", res_xx);
    println!("Result (fx): {}", res_fx);

    Ok(())
}
