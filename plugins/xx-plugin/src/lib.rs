// SPDX-License-Identifier: GPL-3.0-or-later

use hashing::{Hashing, gen_c_wrapper};
use std::hash::Hasher as _;
use twox_hash::XxHash32;

#[derive(Debug, Default)]
pub struct XxPlugin;

impl Hashing for XxPlugin {
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32 {
        let mut hasher = XxHash32::with_seed(seed);
        hasher.write_u32(salt);
        hasher.write(data);
        hasher.finish_32()
    }
}

gen_c_wrapper!(XxPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    const SEED: u32 = 100;
    const SALT: u32 = 42;
    const DATA: &str = "1 if by land, 2 if by sea";
    const TV: u32 = 3263673729;

    #[test]
    fn test_correctness() {
        let mut hasher = XxHash32::with_seed(SEED);
        hasher.write_u32(SALT);
        hasher.write(DATA.as_bytes());
        let it = hasher.finish() as u32;
        assert_eq!(it, TV);
    }

    #[test]
    fn test_plugin() {
        let plugin = XxPlugin::default();
        let result = plugin.hash(SEED, SALT, DATA.as_bytes());
        assert_eq!(result, TV);
    }
}
