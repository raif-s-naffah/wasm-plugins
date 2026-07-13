// SPDX-License-Identifier: GPL-3.0-or-later

use fxhash::FxHasher32;
use hashing::{Hashing, gen_c_wrapper};
use std::hash::Hasher as _;

#[derive(Debug, Default)]
pub struct FxPlugin;

impl Hashing for FxPlugin {
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32 {
        // NOTE (rsn) 20260620 - fxhash does not have a _with_seed() like
        // xxhash.  instead we hash the 'seed' after instantiating a default
        // hasher...
        let mut hasher = FxHasher32::default();
        hasher.write_u32(seed);
        hasher.write_u32(salt);
        hasher.write(data);
        hasher.finish() as u32
    }
}

gen_c_wrapper!(FxPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u32 = 100;
    const SALT: u32 = 42;
    const DATA: &str = "1 if by land, 2 if by sea";
    const TV: u32 = 1779798835;

    #[test]
    fn test_correctness() {
        let mut hasher = FxHasher32::default();
        hasher.write_u32(SEED);
        hasher.write_u32(SALT);
        hasher.write(DATA.as_bytes());
        let it = hasher.finish() as u32;
        assert_eq!(it, TV);
    }

    #[test]
    fn test_plugin() {
        let plugin = FxPlugin::default();
        let result = plugin.hash(SEED, SALT, DATA.as_bytes());
        assert_eq!(result, TV);
    }
}
