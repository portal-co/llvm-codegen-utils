//! # LLVM Codegen Utils Version Macros
//!
//! This crate provides macros for writing version-polymorphic code that works
//! across different LLVM versions.
//!
//! ## Usage
//!
//! The [`vers!`] macro is the primary export. It takes a block of code and a
//! macro name, then expands the code for each enabled LLVM version feature.
//!
//! ## Example
//!
//! ```ignore
//! llvm_codegen_utils_version_macros::vers!({} my_macro);
//!
//! // This expands to:
//! // #[cfg(feature = "llvm-sys-190")] my_macro!(llvm_sys_190 {});
//! // #[cfg(feature = "llvm-sys-180")] my_macro!(llvm_sys_180 {});
//! // ...
//! ```
//!
//! ## Feature Flags
//!
//! The macro generates code conditionally based on these feature flags:
//! - `llvm-sys-180` - LLVM 18 support
//! - `llvm-sys-190` - LLVM 19 support
//! - `llvm-sys-200` - LLVM 20 support
//! - `llvm-sys-210` - LLVM 21 support

include!("macros.rs");
