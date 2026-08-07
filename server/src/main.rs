// SPDX-License-Identifier: GPL-3.0-or-later

use server::{PluginManager, error::WasmPluginError};
use std::time::Instant;

fn main() -> Result<(), WasmPluginError> {
    // the 2 plugin/module IDs...
    const XX_PLUGIN_ID: &str = "xx";
    const FX_PLUGIN_ID: &str = "fx";

    // Test Vectors to ensure algos are working as expected...
    const XX_TV: u32 = 3263673729;
    const FX_TV: u32 = 1779798835;

    let now = Instant::now();

    // we only need 1 of those PER THREAD...
    let mut pm = PluginManager::default();

    // load plugins/modules...
    pm.load(XX_PLUGIN_ID)?;
    pm.load(FX_PLUGIN_ID)?;

    // ready...
    let seed: u32 = 100;
    let salt: u32 = 42;
    let data = "1 if by land, 2 if by sea";

    // let res_xx = pm.call_hash("xx/1", seed, salt, data)?;
    let res_xx = pm.call_hash(XX_PLUGIN_ID, seed, salt, data)?;
    let res_fx = pm.call_hash(FX_PLUGIN_ID, seed, salt, data)?;

    println!("[DEBUG] res_xx = {}", res_xx);
    assert_eq!(res_xx, XX_TV);
    println!("[DEBUG] res_fx = {}", res_fx);
    assert_eq!(res_fx, FX_TV);

    let res_xx2 = pm.call_hash(XX_PLUGIN_ID, seed, salt, data)?;
    println!("[DEBUG] res_xx2 = {}", res_xx2);
    assert_eq!(res_xx, res_xx2);

    let elapsed = now.elapsed();
    println!("Done in {:.3?}", elapsed);

    Ok(())
}
