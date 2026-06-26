// SPDX-License-Identifier: GPL-3.0-or-later

use fxhash::FxHasher32;
use hashing::{Hashing, gen_c_wrapper};
use std::hash::Hasher as _;

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

gen_c_wrapper!("FxPlugin");
