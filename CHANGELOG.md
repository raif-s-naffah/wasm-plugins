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
