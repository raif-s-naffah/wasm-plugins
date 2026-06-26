// SPDX-License-Identifier: GPL-3.0-or-later

use hashing::{Hashing, gen_c_wrapper};
use std::hash::Hasher as _;
use twox_hash::XxHash32;

pub struct XxPlugin;

impl Hashing for XxPlugin {
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32 {
        let mut hasher = XxHash32::with_seed(seed);
        hasher.write_u32(salt);
        hasher.write(data);
        hasher.finish_32()
    }
}

gen_c_wrapper!("XxPlugin");
