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
/// #[cfg(feature = "llvm-sys-XXX")] my_macro!(llvm_sys_XXX { /* contents */ });
/// ```
#[macro_export]
macro_rules! vers {
    ({ $($contents:tt)* } $($m:tt)*) => {
        #[cfg(feature = "llvm-sys-190")] $($m)*! (llvm_sys_190 { $($contents)* });
        #[cfg(feature = "llvm-sys-180")] $($m)*! (llvm_sys_180 { $($contents)* });
        #[cfg(feature = "llvm-sys-200")] $($m)*! (llvm_sys_200 { $($contents)* });
        #[cfg(feature = "llvm-sys-210")] $($m)*! (llvm_sys_210 { $($contents)* });
    };
}
