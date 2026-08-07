// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::OnceLock;

/// Collection of server | application wide sonfiguration parameters.
#[derive(Debug)]
pub(crate) struct MyConfig {
    /// Where the 'plugins' folder, containing the `.wasm` binaries, resides.
    pub(crate) server_dir: String,
    /// Maximum length in bytes of data to process.
    pub(crate) max_payload_len: usize,
    /// Maximum number of plugin instances a Store will hold.
    // forget about trying to build a pool or such like of Plugins.  for this
    // PoC we'll handle 2 distinct plugins, w/ 1 instance each. that's it!
    pub(crate) store_max_instance_count: usize,
    /// Maximum number of memory Pages a Store will allow.
    pub(crate) store_max_page_count: usize,
}

impl Default for MyConfig {
    fn default() -> Self {
        Self {
            server_dir: env!("CARGO_MANIFEST_DIR").to_owned(),
            max_payload_len: 1024,
            store_max_instance_count: 2,
            store_max_page_count: 33,
        }
    }
}

static CONFIG: OnceLock<MyConfig> = OnceLock::new();
pub(crate) fn config() -> &'static MyConfig {
    CONFIG.get_or_init(MyConfig::default)
}
