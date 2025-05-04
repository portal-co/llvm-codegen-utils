#[macro_export]
macro_rules! vers {
    ({ $($contents:tt)* } $($m:tt)*) => {
        #[cfg(feature = "llvm-sys-190")] $($m)*! (llvm_sys_190 { $($contents)* });
        #[cfg(feature = "llvm-sys-180")] $($m)*! (llvm_sys_180 { $($contents)* });
        #[cfg(feature = "llvm-sys-200")] $($m)*! (llvm_sys_200 { $($contents)* });
    };
}
