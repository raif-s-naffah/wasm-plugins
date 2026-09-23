# 2026-09-23

workspace:
* Upgrade dependencies to latest versions (`wasmtime[-wasi]@49.0.0`).

plugins (plugins/xx-plugin):
* Upgrade dependencies to latest versions (`twox-hash@2.1.4`).


# 2026-08-07

workspace:
* Added more + improved comments.

server:
* Find out more about WASM and WASI memory.  Specifically if + how to limit
  memory page-size, page-count, and total memory allocated when loading modules
  and attempting to limit Store's resources grow.
* Output allocated WASM linear memory size when loading a Module.
* Introduce and use a maximum size limit of user data.
* Ensure PluginManager never exceeds plugin and instance count limits.


# 2026-08-01

workspace:
* Upgrade dependencies to latest versions.
* Updated README.

server:
* Scrub used linear memory before returning result.
* add basic verification when loading the a Module re. its 'memory' export.

plugins/xx-plugin:
* fine-tune features list of `twox-hash` dependency.


# 2026-07-13

interfaces/hashing:
* use a Type instead of a Name when calling the gen_c_wrapper macro.
* added unit tests.

plugins/*:
* added unit tests.

server:
* added an error module to group errors raised by this project.
* refactored and exposed a simplified API to use plugins: PluginManager.


# 2026-06-26

* happy birthday.
