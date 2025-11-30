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

/// Generates a markdown table of supported LLVM versions.
fn generate_llvm_version_table() -> String {
    let mut table = String::new();
    table += "| LLVM Version | Feature Flag | llvm-sys Version |\n";
    table += "|--------------|--------------|------------------|\n";
    for (version_id, llvm_sys_version) in LLVMS.iter() {
        // Extract major version (e.g., "190" -> "19", "180" -> "18")
        let major_version = &version_id[..version_id.len() - 1];
        table += &format!(
            "| LLVM {}      | `llvm-sys-{}` | ^{}           |\n",
            major_version, version_id, llvm_sys_version
        );
    }
    table
}

/// Generates a comma-separated list of major LLVM versions.
fn generate_llvm_version_list() -> String {
    LLVMS
        .iter()
        .map(|(v, _)| &v[..v.len() - 1]) // Extract major version
        .collect::<Vec<_>>()
        .join(", ")
}

/// Generates a bullet list of feature flags for rustdoc.
fn generate_feature_flags_doc() -> String {
    LLVMS
        .iter()
        .map(|(version_id, _)| {
            let major_version = &version_id[..version_id.len() - 1];
            format!("//! - `llvm-sys-{}` - LLVM {}", version_id, major_version)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Process a file with GEN markers for LLVM version content.
fn process_file_with_markers(path: &str, _root: &str) -> std::io::Result<()> {
    let s = std::fs::read_to_string(path)?;
    let mut t = String::default();
    let mut in_block_gen = false;
    let mut block_gen_type = String::new();
    
    for l in s.lines() {
        // Handle inline markers like: text <!-- GEN X -->content<!-- RESUME --> more text
        if l.contains("<!-- GEN ") && l.contains("<!-- RESUME -->") && !l.starts_with("<!-- GEN ") {
            let mut result = String::new();
            let mut remaining = l;
            
            while let Some(gen_start) = remaining.find("<!-- GEN ") {
                // Add text before the marker
                result.push_str(&remaining[..gen_start]);
                remaining = &remaining[gen_start..];
                
                // Find the end of the GEN marker
                if let Some(gen_end) = remaining.find(" -->") {
                    let marker_content = &remaining[9..gen_end]; // Skip "<!-- GEN "
                    remaining = &remaining[gen_end + 4..]; // Skip " -->"
                    
                    // Add the marker back
                    result.push_str(&format!("<!-- GEN {} -->", marker_content));
                    
                    // Generate and add new content
                    if marker_content.starts_with("LLVM_VERSION_LIST") {
                        result.push_str(&generate_llvm_version_list());
                    }
                    
                    // Find and skip to RESUME marker
                    if let Some(resume_pos) = remaining.find("<!-- RESUME -->") {
                        result.push_str("<!-- RESUME -->");
                        remaining = &remaining[resume_pos + 15..];
                    }
                } else {
                    result.push_str(remaining);
                    remaining = "";
                }
            }
            result.push_str(remaining);
            t += &result;
            t += "\n";
            continue;
        }
        
        // Handle block start markers
        if let Some(p) = l.strip_prefix("<!-- GEN ") {
            in_block_gen = true;
            block_gen_type = p.trim_end_matches(" -->").to_string();
            t += l;
            t += "\n";
            // Generate content right after the marker
            if block_gen_type.starts_with("LLVM_VERSION_TABLE") {
                t += &generate_llvm_version_table();
            }
            continue;
        }
        
        if let Some(p) = l.strip_prefix("//! <!-- GEN ") {
            in_block_gen = true;
            block_gen_type = p.trim_end_matches(" -->").to_string();
            t += l;
            t += "\n";
            if block_gen_type.starts_with("FEATURE_FLAGS") {
                t += &generate_feature_flags_doc();
                t += "\n";
            }
            continue;
        }
        
        // Handle block end markers
        if l == "<!-- RESUME -->" || l == "//! <!-- RESUME -->" {
            in_block_gen = false;
            block_gen_type.clear();
            t += l;
            t += "\n";
            continue;
        }
        
        // Skip lines inside block generation (they will be regenerated)
        if in_block_gen {
            continue;
        }
        
        t += l;
        t += "\n";
    }
    std::fs::write(path, t)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut args = std::env::args();
    args.next();
    let mut root = args.next().unwrap();
    let mut publish = false;
    if root == "publish" {
        publish = true;
        root = args.next().unwrap();
    }
    
    // Process README.md
    process_file_with_markers(&format!("{root}/README.md"), &root)?;
    
    // Process crate documentation files
    process_file_with_markers(&format!("{root}/crates/llvm-codegen-utils-core/src/lib.rs"), &root)?;
    process_file_with_markers(&format!("{root}/crates/llvm-codegen-utils-version-macros/src/lib.rs"), &root)?;
    
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
