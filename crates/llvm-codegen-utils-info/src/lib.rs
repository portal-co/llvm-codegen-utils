//! # LLVM Codegen Utils Info
//!
//! This crate provides compile-time information about supported LLVM versions.
//!
//! ## Usage
//!
//! The [`LLVMS`] constant contains a mapping of LLVM version identifiers to their
//! corresponding `llvm-sys` crate versions. This is used by the maintenance tool
//! to generate version-specific Cargo.toml entries.
//!
//! ## Example
//!
//! ```
//! use px_llvm_codegen_utils_info::LLVMS;
//!
//! for (llvm_version, llvm_sys_version) in LLVMS {
//!     println!("LLVM {} uses llvm-sys ^{}", llvm_version, llvm_sys_version);
//! }
//! ```

#![no_std]

/// Mapping of LLVM version identifiers to `llvm-sys` crate versions.
///
/// Each tuple contains:
/// - The LLVM major version identifier (e.g., "190" for LLVM 19.0)
/// - The corresponding `llvm-sys` crate version (e.g., "191")
pub static LLVMS: &'static [(&'static str, &'static str)] = &[
    ("190", "191"),
    ("180", "181"),
    ("200", "201"),
    ("210", "211"),
];
