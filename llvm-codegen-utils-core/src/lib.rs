use std::ffi::CStr;
use std::mem::{replace, take, MaybeUninit};
use std::sync::{Arc, Mutex};
use std::thread::LocalKey;
use std::thread_local;

use nonempty::NonEmpty;
mod private {
    pub trait Sealed {}
}

pub trait Ctx: Clone + private::Sealed {}
pub trait Mod: Clone + private::Sealed {
    type Ctx: Ctx;
    fn ctx(&self) -> Self::Ctx;
    fn create_mod(a: &CStr, ctx: &Self::Ctx) -> Self;
}
pub trait Value: Clone + private::Sealed {
    type Tag;
    type Kind: ValueKind<Val<Self::Tag> = Self, Mod = Self::Mod>;
    type Mod: Mod;
    fn r#mod(&self) -> Self::Mod;
}
pub trait ValueKind: private::Sealed {
    type Mod: Mod;
    type Val<K>: Value<Tag = K, Kind = Self, Mod = Self::Mod>;
    type Func: Func<Kind = Self, Mod = Self::Mod>;
}
pub trait Func: Clone + private::Sealed + Value<Tag = FuncTag> {}
pub trait BB: Clone + private::Sealed {}
pub struct LLHandle<K, T>(Arc<LLShim<K, T>>);
impl<K, T> Clone for LLHandle<K, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<K, T> LLHandle<K, T> {
    pub unsafe fn from_raw_parts(ptr: *mut T, dropper: fn(*mut T, K), key: K) -> Self {
        LLHandle(Arc::new(LLShim {
            val: ptr,
            dropper: dropper,
            key: MaybeUninit::new(key),
        }))
    }
    pub fn leaked(ptr: *mut T, key: K) -> Self {
        unsafe { Self::from_raw_parts(ptr, |_, _| {}, key) }
    }
    pub fn ptr(&self) -> *mut T {
        return self.0.val;
    }
    pub fn key(&self) -> &K {
        return unsafe { self.0.key.assume_init_ref() };
    }
}
pub struct LLShim<K, T> {
    val: *mut T,
    key: MaybeUninit<K>,
    dropper: fn(*mut T, K),
}
impl<K, T> Drop for LLShim<K, T> {
    fn drop(&mut self) {
        (self.dropper)(self.val, unsafe {
            replace(&mut self.key, MaybeUninit::uninit()).assume_init()
        })
    }
}
macro_rules! seal {
    ($($t:ty),* $(,)?) => {
        $(impl private::Sealed for $t{})*
    };
}
pub struct Normal;
pub struct FuncTag;
macro_rules! impls {
    ($l:ident {}) => {
        const _: () = {
            use $l as llvm_sys;
            seal!(
                crate::LLHandle<Normal,llvm_sys::LLVMContext>,
                crate::LLHandle<Normal,llvm_sys::LLVMModule>,
                llvm_sys::LLVMValue,
                crate::LLHandle<Normal,llvm_sys::LLVMBasicBlock>
            );
            impl<K> private::Sealed for crate::LLHandle<K, llvm_sys::LLVMValue> {}
            impl<K> crate::Value for crate::LLHandle<K, llvm_sys::LLVMValue> {
                type Tag = K;
                type Kind = llvm_sys::LLVMValue;
                type Mod = crate::LLHandle<Normal, llvm_sys::LLVMModule>;
                fn r#mod(&self) -> Self::Mod {
                    let ptr = self.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMGetGlobalParent(ptr) };
                    crate::LLHandle::leaked(ptr, Normal)
                }
            }
            impl crate::ValueKind for llvm_sys::LLVMValue {
                type Val<K> = crate::LLHandle<K, llvm_sys::LLVMValue>;
                type Mod = crate::LLHandle<Normal, llvm_sys::LLVMModule>;
                type Func = crate::LLHandle<FuncTag, llvm_sys::LLVMValue>;
            }
            impl crate::Ctx for crate::LLHandle<Normal, llvm_sys::LLVMContext> {}
            impl crate::Mod for crate::LLHandle<Normal, llvm_sys::LLVMModule> {
                type Ctx = crate::LLHandle<Normal, llvm_sys::LLVMContext>;
                fn ctx(&self) -> Self::Ctx {
                    let ptr = self.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMGetModuleContext(ptr) };
                    crate::LLHandle::leaked(ptr, Normal)
                }
                fn create_mod(a: &CStr, ctx: &Self::Ctx) -> Self {
                    let ptr = ctx.ptr();
                    let ptr = unsafe {
                        llvm_sys::core::LLVMModuleCreateWithNameInContext(a.as_ptr(), ptr)
                    };
                    unsafe {
                        crate::LLHandle::from_raw_parts(
                            ptr,
                            |a, _| llvm_sys::core::LLVMDisposeModule(a),
                            Normal,
                        )
                    }
                }
            }
            impl crate::Func for crate::LLHandle<FuncTag, llvm_sys::LLVMValue> {}
            impl crate::BB for crate::LLHandle<Normal, llvm_sys::LLVMBasicBlock> {}
        };
    };
}

llvm_codegen_utils_version_macros::vers!({} impls);
