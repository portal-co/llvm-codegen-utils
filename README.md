# LLVM Codegen Utils

A Rust library to facilitate targeting LLVM, providing a mostly safe interface with support for multiple LLVM versions.

> **Note:** Handles currently do not accurately represent their ownership.

## Features

- Safe(r) interface over raw LLVM bindings
- Support for multiple LLVM versions (18, 19, 20, 21)
- Type-safe builders, values, types, and basic blocks
- Automatic resource management through custom handle types

## Supported LLVM Versions

| LLVM Version | Feature Flag | llvm-sys Version |
|--------------|--------------|------------------|
| LLVM 18      | `llvm-sys-180` | ^181           |
| LLVM 19      | `llvm-sys-190` | ^191           |
| LLVM 20      | `llvm-sys-200` | ^201           |
| LLVM 21      | `llvm-sys-210` | ^211           |

## Workspace Structure

This repository is organized as a Cargo workspace containing the following crates:

### `px-llvm-codegen-utils-core`

The core library providing safe abstractions over LLVM. It includes:

- **`Ctx`** - LLVM context wrapper
- **`Mod`** - LLVM module wrapper
- **`Value`** / **`ValueKind`** - Type-safe value representations
- **`Func`** - Function value wrapper
- **`BB`** - Basic block wrapper
- **`Ty`** - Type wrapper with constructors for int, pointer, struct, and function types
- **`Builder`** - IR builder with methods for common instructions (alloca, load, store, arithmetic, branching, etc.)
- **`LLHandle`** - Smart handle type for LLVM resources with automatic cleanup

### `px-llvm-codegen-utils-info`

Contains compile-time information about supported LLVM versions. Exports the `LLVMS` constant which maps LLVM version identifiers to their corresponding `llvm-sys` crate versions.

### `px-llvm-codegen-utils-version-macros`

Provides the `vers!` macro for writing version-polymorphic code that works across different LLVM versions. This macro expands code conditionally based on enabled LLVM version features.

### `llvm-codegen-utils-maintenance`

Internal maintenance tool for managing the workspace. Handles:
- Generating LLVM version-specific Cargo.toml entries
- Synchronizing version numbers across crates
- Publishing crates to crates.io

## Usage

Add the core crate to your `Cargo.toml` with the desired LLVM version feature:

```toml
[dependencies]
px-llvm-codegen-utils-core = { version = "0.1", features = ["llvm-sys-190"] }
```

## Comparison with `inkwell`

| Aspect | inkwell | LLVM Codegen Utils |
|--------|---------|-------------------|
| API Style | Direct LLVM bindings | Higher-level, cleaned interface |
| LLVM Support | LLVM 18 and lower | LLVM 18 and higher |
| Multi-version | Single version per build | Multiple versions via features |

`inkwell` exposes, safely, the actual, non-modified, LLVM API. LLVM Codegen Utils exposes and/or wraps the API to present a safe interface, at the cost of using a slightly higher-level and/or cleaned interface.

## License

This project is licensed under CC0-1.0.