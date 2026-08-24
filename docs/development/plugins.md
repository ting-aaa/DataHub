# Plugin development

DataHub plugins are WebAssembly Components implementing
`wit/datahub-plugin.wit`. The contract receives a JSON-encoded map of declared,
read-only virtual input files and returns bytes for one declared output file.
The host does not link WASI, so plugins receive no ambient filesystem,
environment-variable, credential, clock, random, socket, or network access.

Each package is a directory containing `plugin.toml` and the hash-pinned
component named by `component`:

```toml
id = "example-plugin"
version = "1.0.0"
api_version = "1.0.0"
component = "plugin.wasm"
sha256 = "<64 lowercase hexadecimal characters>"
output_file = "result.bin"

[capabilities]
read_inputs = ["input/data.bin"]
write_output_directory = "generated/example"

[limits]
fuel = 10000000
memory_bytes = 67108864
timeout_ms = 2000
max_input_bytes = 8388608
max_output_bytes = 16777216
```

IDs and paths are validated before installation. Absolute paths, traversal,
backslashes, undeclared inputs, duplicate capabilities, symlinks, hash changes,
and incompatible API versions are rejected. Installation is immutable and
selected by exact semantic version. Runtime limits cover fuel, wall-clock time,
linear memory, input bytes, and output bytes.

The repository example is compiled and componentized by the local quality gate:

```powershell
rustup target add wasm32-unknown-unknown --toolchain 1.96.0
cargo build --manifest-path examples/datahub-echo-plugin/Cargo.toml --target wasm32-unknown-unknown --release
cargo run -p datahub-plugin-host --example componentize -- <core.wasm> <plugin.wasm>
cargo run -p datahub-plugin-host -- run-package <package-directory> hello
```

All Python helpers, if later added, must run through `uv`.
