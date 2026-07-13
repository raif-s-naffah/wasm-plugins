// SPDX-License-Identifier: GPL-3.0-or-later

//! _Plugins_ offering their own flavour of a corresponding _Interface_ are
//! expected to implement this trait.

/// Expose the capability to digest a byte slice using a built-in algorithm.
pub trait Hashing {
    /// One shot call to instantiate + seed a hashing algorithm, then hash in sequence
    /// a salt, followed by a slice of bytes.
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32;
}

#[macro_export]
macro_rules! gen_c_wrapper {
    ($plugin_type:ty) => {
        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(no_mangle)]
        pub extern "C" fn hash(seed: u32, salt: u32, offset: *const u8, length: usize) -> u32 {
            let data = unsafe { ::std::slice::from_raw_parts(offset, length) };
            let z_plugin = <$plugin_type>::default();
            z_plugin.hash(seed, salt, data)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct OkPlugin;

    impl Hashing for OkPlugin {
        fn hash(&self, seed: u32, salt: u32, _data: &[u8]) -> u32 {
            seed.wrapping_add(salt)
        }
    }

    #[derive(Debug, Default)]
    struct ErrPlugin;

    impl Hashing for ErrPlugin {
        fn hash(&self, _seed: u32, _salt: u32, _data: &[u8]) -> u32 {
            0
        }
    }

    #[test]
    fn test_ok_plugin_success() {
        let plugin = OkPlugin::default();
        let result = plugin.hash(15, 27, "whatever...".as_bytes());
        assert_eq!(result, 42);
    }

    #[test]
    fn test_err_plugin_failure() {
        let plugin = ErrPlugin::default();
        let result = plugin.hash(15, 27, "whatever...".as_bytes());
        assert_eq!(result, 0);
    }
}
