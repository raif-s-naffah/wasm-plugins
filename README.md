# WASM-plugins
Proof of Concept for creating and using [WASM](https://webassembly.org/) plugins from Rust apps/servers

## Why
This project demonstrates how can Rust plugins packaged as [WebAssembly System Interface (WASI) Preview 1](https://github.com/WebAssembly/WASI) be built and used by a "server" logic that loads + use them at runtime; i.e. no compile-time knowledge except for the _Trait(s)_ they're supposed to implement, and where their WASM files are to be found.

Normally you would use an Interface Definition Language (IDL) to specify the _Interface(s)_ those plugins are supposed to implement, generate language specific bindings for them, and later in the server/app that will be using them, load + interact w/ them through some runtime layer.  For WASI, the IDL is the WIT (WASM Interface Type) language.  A [`wit-bindgen`](https://crates.io/crates/wit-bindgen) crate[^1], and [`wit-bindgen-cli`](https://crates.io/crates/wit-bindgen-cli)[^2], should theoretically generate working Rust traits and macros to use. I however was not able to use those resources to generate and re-use a clean Rust crate representing the _Interface_, instead i hand-crafted those learning from the generated source produced by the CLI tool.  The WIT source looked like this...

```text
package services:api@0.1.0;

interface hashing {
  hash: func(seed: u32, salt: u32, data: string) -> u32;
}

world lib {
  export hashing;
}
```

## Organization
This project is organized as a Cargo Workspace. One _Interface_ crate is defined in `interfaces/hashing`, and named _Hashing_:

```rust
pub trait Hashing {
    fn hash(&self, seed: u32, salt: u32, data: &[u8]) -> u32;
}
```

It's sole function is a one-shot call to instantiate a hashing algorithm with a provided _seed_, then digest a _salt_ (an unsigned 32-bit integer), followed by a byte slice to produce + return an unsigned 32-bit integer result.

Two implementations using [`fxhash`](https://crates.io/crates/fxhash) and [`twox-hash`](https://crates.io/crates/twox-hash) algorithms are provided in crates `plugins/fx-plugin` and `plugins/xx-plugin` respectively.


## Prerequisites
This project builds the plugins using the `cargo build` command with the target `wasm32-wasip1`. That target must be installed beforehand w/

`$ rustup target add wasm32-wasip1 ↵`.


## How to build + use
A Bourne Again SHell (`build-plugins.sh`) is provided in the project's root directory to compile both plugins and copy the resulting WASM files to a folder named `plugins` inside the `server`. 
The `main()` function of `server` expects the plugins to be found there.

From the project's root, do

`$ ./build-plugins.sh ↵`.

Once the plugins are built (and copied to the server's plugins folder), do

`$ cargo r -p server -r ↵`.

The `main()` function of the `server` loads, then calls both plugins w/ the same data and prints the results.


## License
This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 



[^1]: a _"...Guest Rust language bindings generator for WIT and the Component Model"_
[^2]: a Command Line Interface tool using the same crate.
