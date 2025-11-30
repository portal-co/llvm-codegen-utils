//! # LLVM Codegen Utils Maintenance
//!
//! Internal maintenance tool for managing the llvm-codegen-utils workspace.
//!
//! ## Purpose
//!
//! This tool handles:
//! - Generating LLVM version-specific Cargo.toml dependency entries
//! - Synchronizing version numbers across all crates
//! - Generating the `vers!` macro implementation
//! - Publishing crates to crates.io (with `publish` argument)
//!
//! ## Usage
//!
//! ```bash
//! # Run maintenance tasks
//! cargo run -p llvm-codegen-utils-maintenance -- /path/to/workspace
//!
//! # Publish all crates
//! cargo run -p llvm-codegen-utils-maintenance -- publish /path/to/workspace
//! ```
//!
//! ## Cargo.toml Markers
//!
//! The tool processes special comment markers in Cargo.toml files:
//!
//! - `# GEN LLVM` / `# RESUME` - Generate LLVM dependency entries
//! - `# GEN LL_FEATURES` / `# RESUME` - Generate LLVM feature flags
//! - `# GEN LL_DEPS` / `# RESUME` - Generate cross-crate LLVM dependencies
//! - `# GEN VERSION` / `# RESUME` - Synchronize version from version.txt

use std::{fs::FileType, iter::once, path::PathBuf, sync::LazyLock};

use itertools::Itertools;
use llvm_codegen_utils_info::LLVMS;
use quasiquote::quasiquote;
use quote::format_ident;

fn main() -> std::io::Result<()> {
    let mut args = std::env::args();
    args.next();
    let mut root = args.next().unwrap();
    let mut publish = false;
    if root == "publish" {
        publish = true;
        root = args.next().unwrap();
    }
    let s = std::fs::read_to_string(format!("{root}/Cargo.toml"))?;
    let mut t = String::default();
    let mut generating = false;
    for l in s.lines() {
        if let Some(p) = l.strip_prefix("# GEN ") {
            generating = true;
            t += l;
            t += "\n";
            if p.starts_with("LLVM") {
                for (a, b) in LLVMS.iter() {
                    t += &format!("llvm-sys-{a}={{version=\"^{b}\",package=\"llvm-sys\"}}\n");
                }
            }
        }
        if l.starts_with("# RESUME") {
            generating = false;
        }
        if generating {
            continue;
        }
        t += l;
        t += "\n";
    }
    std::fs::write(format!("{root}/Cargo.toml"), t)?;
    let ver = std::fs::read_to_string(format!("{root}/version.txt"))?;
    for f in std::fs::read_dir(&format!("{root}/crates"))? {
        let Ok(f) = f else {
            continue;
        };
        if f.file_name().as_encoded_bytes().iter().all(|a| *a == b'.') {
            continue;
        }
        if f.file_type()?.is_dir() {
            cargo(f.path(), &ver)?;
        }
    }
    let xs = LLVMS.iter().map(|(a, _)| {
        quasiquote! {
            #[cfg(feature = #{format!("llvm-sys-{a}")})]
            $($m)*!(#{format_ident!("llvm_sys_{a}")} {$($contents)*} )
        }
    });
    let contents = quasiquote! {
        /// Macro for writing version-polymorphic code across LLVM versions.
        ///
        /// This macro expands code conditionally based on enabled LLVM version features.
        /// It takes a block of content and a macro name, then invokes the macro for each
        /// enabled LLVM version with the appropriate `llvm_sys_*` module identifier.
        ///
        /// # Usage
        ///
        /// ```ignore
        /// vers!({/* contents */} my_macro);
        /// ```
        ///
        /// # Expansion
        ///
        /// For each enabled LLVM version feature, this expands to:
        /// ```ignore
        /// #[cfg(feature = "llvm-sys-190")] my_macro!(llvm_sys_190 { /* contents */ });
        /// #[cfg(feature = "llvm-sys-180")] my_macro!(llvm_sys_180 { /* contents */ });
        /// // ... and so on for other enabled versions
        /// ```
        #[macro_export]
        macro_rules! vers{
            ({$($contents:tt)*} $($m:tt)*) => {
                #(#xs);*;
            }
        }
    };
    std::fs::write(
        format!("{root}/crates/llvm-codegen-utils-version-macros/src/macros.rs"),
        prettyplease::unparse(&syn::parse2(contents).unwrap()),
    )?;
    if publish {
        if !std::process::Command::new("git")
            .arg("add")
            .arg("-A")
            .current_dir(&root)
            .spawn()?
            .wait()?
            .success()
        {
            panic!("command failed")
        };
        std::process::Command::new("git")
            .arg("commit")
            .arg("-m")
            .arg("publish cleanup")
            .current_dir(&root)
            .spawn()?
            .wait()?;
        for f in std::fs::read_dir(&format!("{root}/crates"))? {
            let Ok(f) = f else {
                continue;
            };
            if f.file_name().as_encoded_bytes().iter().all(|a| *a == b'.') {
                continue;
            }
            if !f.file_type()?.is_dir() {
                continue;
            }
            if !f.path().join("Cargo.toml").exists() {
                continue;
            }
            match f.file_name().to_str() {
                Some("llvm-codegen-utils-maintenance") => continue,
                _ => {}
            };
            if !std::process::Command::new("cargo")
                .arg("publish")
                .current_dir(f.path())
                .spawn()?
                .wait()?
                .success()
            {
                panic!("publish of {} failed", f.file_name().to_string_lossy())
            }
        }
    }
    Ok(())
}

fn cargo(root: PathBuf, ver: &str) -> std::io::Result<()> {
    let p = root.join("Cargo.toml");
    if !p.exists() {
        return Ok(());
    }
    let s = std::fs::read_to_string(&p)?;
    let deps =
        std::fs::read_to_string(&root.join("llvm-deps.list")).unwrap_or_else(|_| format!(""));
    let mut t = String::default();
    let mut generating = false;
    for l in s.lines() {
        if let Some(p) = l.strip_prefix("# GEN ") {
            generating = true;
            t += l;
            t += "\n";
            if p.starts_with("LLVM") {
                for (a, b) in LLVMS.iter() {
                    t += &format!("llvm-sys-{a}={{workspace=true,optional=true}}\n");
                }
            }
            if p.starts_with("LL_FEATURES") {
                for (a, b) in LLVMS.iter() {
                    let x = once(format!("\"dep:llvm-sys-{a}\""))
                        .chain(deps.lines().map(|l| format!("{l}/llvm-sys-{a}")))
                        .join(",");
                    t += &format!("llvm-sys-{a}=[{x}]\n");
                }
            }
            if p.starts_with("LL_DEPS") {
                for d in deps
                    .lines()
                    .chain(once("llvm-codegen-utils-version-macros"))
                {
                    t += &format!(
                        r#"{d} = {{ version = "{ver}", path = "../{d}", package = "px-{d}" }}{}"#,
                        "\n"
                    )
                }
            }
            if p.starts_with("VERSION") {
                t += &format!("version = \"{ver}\"\n")
            }
        }
        if l.starts_with("# RESUME") {
            generating = false;
        }
        if generating {
            continue;
        }
        t += l;
        t += "\n";
    }
    std::fs::write(&p, t)?;
    Ok(())
}
