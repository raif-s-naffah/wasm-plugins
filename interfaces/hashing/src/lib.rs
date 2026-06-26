// SPDX-License-Identifier: GPL-3.0-or-later

//! Rust trait (aka _Interface_) _Plugins_ are expected to implement to offer
//! their flavour of such "service".

/// One shot call to instantiate + seed a hashing algorithm, then hash in sequence
/// a salt, followed by a slice of bytes.
pub trait Hashing {
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32;
}

/// Macro to generate the C wrapper for the interface single function. Only
/// required parameter is the name of the `struct` that will represent the
/// plugin's instance.
#[macro_export]
macro_rules! gen_c_wrapper {
    ($name:expr) => {
        ::paste::paste! {
            #[allow(clippy::not_unsafe_ptr_arg_deref)]
            #[unsafe(no_mangle)]
            pub extern "C" fn hash(seed: u32, salt: u32, ptr: *const u8, len: usize) -> u32 {
                unsafe {
                    let data = ::std::slice::from_raw_parts(ptr, len);
                    let plugin = [<$name>];

                    let result = plugin.hash(seed, salt, data);
                    println!("{}::hash({}, {}, ...): {}", $name, seed, salt, result);
                    result
                }
            }
        }
    };
}
