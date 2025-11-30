//! # LLVM Codegen Utils Core
//!
//! This crate provides safe abstractions over LLVM's C API for code generation.
//!
//! ## Overview
//!
//! The core abstractions include:
//!
//! - [`Ctx`] - LLVM context wrapper
//! - [`Mod`] - LLVM module wrapper for organizing functions and global values
//! - [`Value`] / [`ValueKind`] - Type-safe representations of LLVM values
//! - [`Func`] - Function value wrapper
//! - [`BB`] - Basic block wrapper for control flow
//! - [`Ty`] - LLVM type wrapper with constructors for common types
//! - [`Builder`] - IR builder providing methods for instruction generation
//! - [`LLHandle`] - Smart handle for LLVM resources with automatic cleanup
//!
//! ## LLVM Version Support
//!
//! This crate supports multiple LLVM versions through feature flags:
//!
//! <!-- GEN FEATURE_FLAGS -->
//! - `llvm-sys-190` - LLVM 19
//! - `llvm-sys-180` - LLVM 18
//! - `llvm-sys-200` - LLVM 20
//! - `llvm-sys-210` - LLVM 21
//! <!-- RESUME -->
//!
//! Enable exactly one feature flag corresponding to your installed LLVM version.

use std::collections::BTreeMap;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::{replace, take, transmute, MaybeUninit};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::LocalKey;
use std::thread_local;

use nonempty::NonEmpty;
use typenum::Same;
mod private {
    pub trait Sealed {}
}

/// Trait for LLVM context wrappers.
///
/// The context is the top-level container for LLVM's internal state.
pub trait Ctx<'a>: Clone + private::Sealed + 'a {}

/// Trait for LLVM module wrappers.
///
/// A module contains functions, global variables, and other top-level definitions.
pub trait Mod<'a>: Clone + private::Sealed + 'a {
    /// The context type associated with this module.
    type Ctx<'b>: Ctx<'b>
    where
        Self: 'b;
    /// Returns the context this module belongs to.
    fn ctx<'b: 'a>(&'b self) -> Self::Ctx<'b>;
    /// Creates a new module with the given name in the specified context.
    fn create_mod<'b, 'c, 'd>(a: &'b CStr, ctx: &'c Self::Ctx<'d>) -> Self
    where
        'a: 'b + 'c + 'd;
}

/// Trait for LLVM value wrappers.
///
/// Values represent computed results (constants, function arguments, instructions, etc.).
pub trait Value<'a>: Clone + private::Sealed + 'a {
    /// The tag type used to distinguish value categories.
    type Tag: 'a;
    /// The value kind associated with this value.
    type Kind: for<'b> ValueKind<Val<'a, Self::Tag> = Self, Mod<'b> = Self::Mod<'b>>;
    /// The module type this value belongs to.
    type Mod<'b>: Mod<'b>;
    /// Returns the module this value belongs to.
    fn r#mod<'b: 'a>(&'b self) -> Self::Mod<'b>;
}

/// Trait for classifying LLVM value kinds.
///
/// This provides factory methods for creating values of specific types.
pub trait ValueKind: private::Sealed {
    /// The module type.
    type Mod<'a>: Mod<'a>;
    /// The value type parameterized by a tag.
    type Val<'a, K: 'a>: for<'b> Value<'a, Tag = K, Kind = Self, Mod<'b> = Self::Mod<'b>>
    where
        K: 'a;
    /// The function type.
    type Func<'a>: for<'b> Func<'a, Kind = Self, Mod<'b> = Self::Mod<'b>>;
    /// The LLVM type wrapper.
    type Ty<'a>: Ty<'a>;
    /// Creates a constant integer value.
    fn const_int<'a>(ty: Self::Ty<'a>, n: u64, sext: bool) -> Self::Val<'a, Normal>;
    /// Adds a function to the module.
    fn function<'a, 'b, 'c, 'd: 'a + 'b + 'c>(
        r#mod: Self::Mod<'a>,
        name: &'b CStr,
        ty: Self::Ty<'c>,
    ) -> Self::Func<'d>;
}

/// Trait for LLVM function value wrappers.
pub trait Func<'a>: Clone + private::Sealed + Value<'a, Tag = FuncTag> + 'a {}

/// Trait for LLVM basic block wrappers.
///
/// Basic blocks are sequences of instructions with a single entry and single exit.
pub trait BB<'a>: Clone + private::Sealed + 'a {
    /// The function type this basic block belongs to.
    type Func<'b>: Func<'b>
    where
        'a: 'b,
        Self: 'b;
    /// Creates a new basic block in the given function.
    fn new<'b, 'c>(f: Self::Func<'b>, name: &'c CStr) -> Self
    where
        'a: 'b + 'c;
}
macro_rules! rest {
    ($llvm:ident as [$i:ident ($(($l:lifetime) @ $e:ident : $t:ty as |$v:ident|$b:expr),*)]) => {
        $(let $e= match $e{$v => $b});*;
        paste::paste!{

        }
    };
}
macro_rules! inst {
    (($l2:lifetime)@ [$($a:tt)*] => $($b:tt)*) => {
        inst!(($l2) @ $($a)* => $($b)* => [$($a)*]);
    };
    (($l2:lifetime)@ $(#[$doc:meta])* $i:ident ($(($($l:lifetime),*) @ $e:ident : $t:ty as |$v:ident|$b:expr),*) =>  $($llvm:ident )? => $stuff:tt) => {
        paste::paste!{
            $(#[$doc])*
            #[allow(unreachable_code,unused_variables)]
            fn $i<'b,$($($l),*),*,'res:  $($($l +)* )* 'b>(&'b self, $($e: $t),*) -> <Self::ValKind<'a,'a> as ValueKind>::Val<'res,Normal> where $($($l2 : $l),*),*{

                let builder = |(),$($e : $t),*| -> std::convert::Infallible{
                    panic!("abstract method used")
                };
                let ptr = ();
                let leaked = |a: std::convert::Infallible,b: Normal| match a{};
                let mark: Result<std::convert::Infallible,()> = Err(());
                // macro_rules! shim{
                //     () => {

                //     };
                // }
                $(
                    rest!($llvm as $stuff);
                    let builder = $llvm::core::[<LLVMBuild $i >];
                    let ptr = self.ptr();
                    let leaked = |a,b|unsafe{crate::LLHandle::leaked(a,b)};
                    let mark: Result<(),std::convert::Infallible> = Ok(());

                    // shim!()
                )?;

                let res = unsafe{
                    builder(ptr,$($e),*)
                };
                leaked(res,Normal)
            }
        }
    };
}
macro_rules! insts {
    (($l2:lifetime)@{[$(#[$doc:meta])* $i:ident $($t0:tt)*], $([$($t:tt)*],)*} => $(<$llvm:ident>)?) => {
        inst!(($l2)@[$(#[$doc])* $i $($t0)*] => $($llvm)?);
        insts!(($l2)@{$([$($t)*],)*} => $(<$llvm>)?);
    };
    (($l2:lifetime)@{} => $(<$llvm:ident>)?) => {

    };
}
/// Integer comparison predicates for use with [`Builder::ICmp`].
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum ICmp {
    /// Equal comparison.
    Eq,
    /// Unsigned less-than comparison.
    Lt,
    /// Signed less-than comparison.
    Lts,
}
macro_rules! default_insts {
    ($l2:lifetime @ $($llvm:ident)?) => {
        insts!(($l2) @ {
            [
                /// Allocates memory on the stack for a value of the given type.
                ///
                /// Returns a pointer to the allocated memory.
                #[doc = ""]
                /// # Parameters
                /// - `ty`: The type to allocate space for
                /// - `name`: Name for the resulting instruction
                Alloca (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Loads a value from memory.
                ///
                /// # Parameters
                /// - `ty`: The type of the value to load
                /// - `pointer`: Pointer to the memory location to load from
                /// - `name`: Name for the resulting instruction
                Load2 (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Gets a pointer to a struct field.
                ///
                /// # Parameters
                /// - `ty`: The type of the struct
                /// - `pointer`: Pointer to the struct
                /// - `idx`: Index of the field to get a pointer to
                /// - `name`: Name for the resulting instruction
                StructGEP2 (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr(), ('idx) @ idx: &'idx u32 as |x|*x, ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Stores a value to memory.
                ///
                /// # Parameters
                /// - `value`: The value to store
                /// - `pointer`: Pointer to the memory location to store to
                Store (('val) @ value: <Self::ValKind<'a,'a> as ValueKind>::Val<'val,Normal> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr())
            ],
            [
                /// Adds two integer values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                Add (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Performs bitwise AND on two values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                And (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Negates an integer value (two's complement).
                ///
                /// # Parameters
                /// - `lhs`: The value to negate
                /// - `name`: Name for the resulting instruction
                Neg (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(),  ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Performs bitwise NOT on a value.
                ///
                /// # Parameters
                /// - `lhs`: The value to invert
                /// - `name`: Name for the resulting instruction
                Not (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Truncates or bitcasts a value to a different type.
                ///
                /// # Parameters
                /// - `lhs`: The value to convert
                /// - `ty`: The target type
                /// - `name`: Name for the resulting instruction
                TruncOrBitCast (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(),('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Multiplies two integer values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                Mul (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Performs bitwise OR on two values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                Or (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Subtracts two integer values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                Sub (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Performs bitwise XOR on two values.
                ///
                /// # Parameters
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                Xor (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Performs an integer comparison.
                ///
                /// # Parameters
                /// - `op`: The comparison predicate (see [`ICmp`])
                /// - `lhs`: Left-hand side operand
                /// - `rhs`: Right-hand side operand
                /// - `name`: Name for the resulting instruction
                ICmp (('op) @ op: crate::ICmp as |a|a.into(),('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())
            ],
            [
                /// Unconditional branch to a basic block.
                ///
                /// # Parameters
                /// - `dest`: The target basic block
                Br (('dest) @ dest: Self::BB<'dest,'a,'a> as |x|x.ptr())
            ],
            [
                /// Conditional branch based on an i1 value.
                ///
                /// # Parameters
                /// - `if`: The condition (must be i1 type)
                /// - `then`: Basic block to branch to if condition is true
                /// - `else`: Basic block to branch to if condition is false
                CondBr (('cond) @ r#if: <Self::ValKind<'a,'a> as ValueKind>::Val<'cond,Normal> as |x|x.ptr(), ('then) @ then: Self::BB<'then,'a,'a> as |x|x.ptr(),('e) @ r#else: Self::BB<'e,'a,'a> as |x|x.ptr())
            ],
        } => $(<$llvm>)?);
    };
}
/// Trait for LLVM type wrappers.
///
/// Provides constructors for common LLVM types: integers, pointers, structs, and functions.
pub trait Ty<'a>: Clone + private::Sealed + 'a {
    /// The context type associated with this type.
    type Ctx<'b>: Ctx<'b>
    where
        Self: 'b;
    /// Creates an integer type with the specified bit width.
    fn int_ty(ctx: Self::Ctx<'a>, size: u32) -> Self;
    /// Creates a pointer type in the specified address space.
    fn ptr_ty(ctx: Self::Ctx<'a>, address_space: u32) -> Self;
    /// Creates a struct type with the specified field types.
    fn struct_ty(ctx: Self::Ctx<'a>, fields: impl Iterator<Item = Self>, packed: bool) -> Self;
    /// Creates a function type with this type as the return type.
    fn fun_ty(self, params: impl Iterator<Item = Self>) -> Self;
}

/// Trait for LLVM IR builder wrappers.
///
/// The builder provides methods for generating LLVM IR instructions.
/// It maintains a current insertion point (basic block) and provides
/// methods for common operations like arithmetic, memory access, and control flow.
///
/// # Supported Instructions
///
/// Instructions are generated via trait methods. Method names follow LLVM's
/// naming convention (PascalCase) for macro-generated methods, while manually
/// defined methods use snake_case:
///
/// - **Memory**: `Alloca`, `Load2`, `Store`, `StructGEP2`, `gep2`
/// - **Arithmetic**: `Add`, `Sub`, `Mul`, `Neg`
/// - **Bitwise**: `And`, `Or`, `Xor`, `Not`
/// - **Comparison**: `ICmp`
/// - **Conversion**: `TruncOrBitCast`
/// - **Control Flow**: `Br`, `CondBr`
/// - **Calls**: `call`
pub trait Builder<'a>: Clone + private::Sealed + 'a {
    /// The basic block type for this builder.
    type BB<'b, 'e, 'd>: BB<'b, Func<'b>: Value<'b, Kind = Self::ValKind<'e, 'd>>>
    where
        Self: 'b,
        'a: 'b,
        Self: 'e,
        Self: 'd;
    type ValKind<'d, 'b>: ValueKind<Ty<'d> = Self::Ty<'d>, Mod<'b> = Self::Mod<'b>>
    where
        Self: 'd,
        Self: 'b;
    // type InternalValShim<'b: 'a, 'd, 'e, K: 'b>: Value<'b, Tag = K>
    //     + Same<Output = <Self::ValKind<'d, 'e> as ValueKind>::Val<'b, K>>
    // where
    //     Self: 'd,
    //     Self: 'e;
    // type Val<'b: 'a, K: 'b>: Value<'b, Tag = K>
    //     + for<'d, 'e> Into<Self::InternalValShim<'b, 'd, 'e, K>>
    //     + for<'d, 'e> From<Self::InternalValShim<'b, 'd, 'e, K>>;
    // type Val<'b, K: 'b>: Value<'b, Tag = K>
    //     + for<'d, 'e> Same<Output = <Self::ValKind<'d, 'e> as ValueKind>::Val<'b, K>>;
    type Mod<'b>: Mod<'b, Ctx<'b> = Self::Ctx<'b>>
    where
        Self: 'b;
    type Ty<'b>: Ty<'b>
    where
        Self: 'b;
    type Ctx<'b>: Ctx<'b>
    where
        Self: 'b;
    fn new_in_ctx(ctx: Self::Ctx<'a>) -> Self;
    fn r#continue<'b, 'c>(&'b self, bb: Self::BB<'c, '_, '_>)
    where
        'a: 'b + 'c;
    fn call<'b, 'c, 'd, 'e, 'f, 'h, 'i, 'g: 'a + 'b + 'c + 'd + 'e + 'f + 'h + 'i>(
        &'b self,
        resty: Self::Ty<'c>,
        r#fn: <Self::ValKind<'_, '_> as ValueKind>::Val<'d, Normal>,
        args: impl Iterator<Item = <Self::ValKind<'h, 'i> as ValueKind>::Val<'e, Normal>>,
        name: &'f CStr,
    ) -> <Self::ValKind<'_, '_> as ValueKind>::Val<'g, Normal>
    where
        Self: 'h + 'i;
    fn gep2<'b, 'c, 'd, 'e, 'f, 'h, 'i, 'g: 'a + 'b + 'c + 'd + 'e + 'f + 'h + 'i>(
        &'b self,
        resty: Self::Ty<'c>,
        ptr: <Self::ValKind<'_, '_> as ValueKind>::Val<'d, Normal>,
        args: impl Iterator<Item = <Self::ValKind<'h, 'i> as ValueKind>::Val<'e, Normal>>,
        name: &'f CStr,
    ) -> <Self::ValKind<'_, '_> as ValueKind>::Val<'g, Normal>
    where
        Self: 'h + 'i;
    default_insts!('a @ );
}
static M: LazyLock<Mutex<BTreeMap<usize, (usize, Box<dyn FnOnce(*mut (), *mut ()) + Send>)>>> =
    LazyLock::new(|| Default::default());

/// A smart handle for LLVM resources.
///
/// This type provides reference counting and automatic cleanup for LLVM resources.
/// When the last clone of a handle is dropped, the associated LLVM resource is
/// disposed of via the dropper function provided during construction.
///
/// # Type Parameters
///
/// - `'a` - Lifetime of the handle
/// - `K` - Key/tag type used to categorize the resource
/// - `T` - The underlying LLVM type being wrapped
///
/// # Safety
///
/// This type uses unsafe internally to manage raw pointers to LLVM resources.
/// Callers must ensure the underlying LLVM resources remain valid for the
/// lifetime of the handle.
pub struct LLHandle<'a, K, T> {
    val: *mut T,
    key: *mut K,
    phantom: PhantomData<fn(K, &'a T) -> (K, &'a T)>,
}
impl<'a, K, T> Clone for LLHandle<'a, K, T> {
    fn clone(&self) -> Self {
        if let Some((n, _)) = M.lock().unwrap().get_mut(&(self.val as usize)) {
            *n += 1;
        }
        Self {
            val: self.val.clone(),
            key: self.key.clone(),
            phantom: self.phantom.clone(),
        }
    }
}
impl<'a, K, T> Drop for LLHandle<'a, K, T> {
    fn drop(&mut self) {
        let mut lock = M.lock().unwrap();
        let Some((a, b)) = lock.remove(&(self.val as usize)) else {
            return;
        };
        if a == 0 {
            drop(lock);
            b(self.val.cast(), self.key.cast());
        } else {
            lock.insert(self.val as usize, (a - 1, b));
        }
    }
}
impl<'a, K, T> LLHandle<'a, K, T> {
    /// Creates a new handle from raw parts with automatic cleanup.
    ///
    /// # Safety
    ///
    /// - `ptr` must be a valid pointer to an LLVM resource
    /// - `dropper` must properly dispose of the resource when called
    /// - The resource must not be disposed of by any other means
    pub unsafe fn from_raw_parts(ptr: *mut T, dropper: fn(*mut T, K), key: K) -> Self {
        let key = Box::into_raw(Box::new(key));
        M.lock().unwrap().insert(
            ptr as usize,
            (
                0,
                match Box::new(move |b: *mut (), a: *mut ()| {
                    let key = a as *mut K;
                    let ptr = b as *mut T;
                    dropper(ptr.cast(), *unsafe { Box::from_raw(key) });
                }) {
                    val => {
                        let val: Box<dyn FnOnce(*mut (), *mut ()) + Send + '_> = val;
                        unsafe { transmute(val) }
                    }
                },
            ),
        );
        Self {
            val: ptr,
            key,
            phantom: PhantomData,
        }
    }

    /// Creates a handle for a "leaked" resource that won't be automatically cleaned up.
    ///
    /// Use this for LLVM resources that are owned by another resource
    /// (e.g., values owned by their parent module).
    ///
    /// # Safety
    ///
    /// - `ptr` must be a valid pointer to an LLVM resource
    /// - The resource must outlive the handle
    pub unsafe fn leaked(ptr: *mut T, key: K) -> Self {
        Self {
            val: ptr,
            key: Box::into_raw(Box::new(key)),
            phantom: PhantomData,
        }
    }

    /// Returns the raw pointer to the underlying LLVM resource.
    pub fn ptr(&self) -> *mut T {
        return self.val;
    }

    /// Returns a reference to the key/tag associated with this handle.
    pub fn key(&self) -> &K {
        return unsafe { &*self.key };
    }
}

macro_rules! seal {
    ($(<$($generics:lifetime),*> => $t:ty),* $(,)?) => {
        $(impl<$($generics),*> private::Sealed for $t{})*
    };
}

/// Marker type for normal (non-function) LLVM values.
pub struct Normal;

/// Marker type for function LLVM values.
pub struct FuncTag;
macro_rules! impls {
    ($l:ident {}) => {
        const _: () = {
            use $l as llvm_sys;
            seal!(
             <'a>  =>   crate::LLHandle<'a,Normal,llvm_sys::LLVMContext>,
              <'a>  =>  crate::LLHandle<'a,Normal,llvm_sys::LLVMModule>,
               <> =>  llvm_sys::LLVMValue,
              <'a>  =>  crate::LLHandle<'a,Normal,llvm_sys::LLVMBasicBlock>,
              <'a>  =>  crate::LLHandle<'a,Normal,llvm_sys::LLVMBuilder>,
              <'a>  =>  crate::LLHandle<'a,Normal,llvm_sys::LLVMType>,
            );
            impl From<crate::ICmp> for llvm_sys::LLVMIntPredicate{
                fn from(a: crate::ICmp) -> Self{
                    match a{
                        crate::ICmp::Eq => llvm_sys::LLVMIntPredicate::LLVMIntEQ,
                        crate ::ICmp::Lt => llvm_sys::LLVMIntPredicate::LLVMIntULT,
                        crate ::ICmp::Lts => llvm_sys::LLVMIntPredicate::LLVMIntSLT,
                    }
                }
            }
            impl<'a, K> private::Sealed for crate::LLHandle<'a, K, llvm_sys::LLVMValue> {}
            impl<'a, K: 'a> crate::Value<'a> for crate::LLHandle<'a, K, llvm_sys::LLVMValue> {
                type Tag = K;
                type Kind = llvm_sys::LLVMValue;
                type Mod<'b> = crate::LLHandle<'b, Normal, llvm_sys::LLVMModule>;
                fn r#mod<'b: 'a>(&'b self) -> Self::Mod<'b> {
                    let ptr = self.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMGetGlobalParent(ptr) };
                    unsafe { crate::LLHandle::leaked(ptr, Normal) }
                }
            }
            impl<'a> crate::Ty<'a> for crate::LLHandle<'a, Normal, llvm_sys::LLVMType> {
                type Ctx<'b>
                    = crate::LLHandle<'b, Normal, llvm_sys::LLVMContext>
                where
                    Self: 'b;
                fn int_ty(ctx: Self::Ctx<'a>, size: u32) -> Self {
                    let ptr = ctx.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMIntTypeInContext(ptr, size) };
                    unsafe { LLHandle::leaked(ptr, Normal) }
                }
                fn ptr_ty(ctx: Self::Ctx<'a>, address_space: u32) -> Self {
                    let ptr = ctx.ptr();
                    let ptr =
                        unsafe { llvm_sys::core::LLVMPointerTypeInContext(ptr, address_space) };
                    unsafe { LLHandle::leaked(ptr, Normal) }
                }
                fn fun_ty(self, params: impl Iterator<Item = Self>) -> Self {
                    let ptr = self.ptr();
                    let mut args = params.map(|p| p.ptr()).collect::<Vec<_>>();
                    let ptr = unsafe {
                        llvm_sys::core::LLVMFunctionType(
                            ptr,
                            args.as_mut_ptr(),
                            args.len().try_into().unwrap(),
                            0,
                        )
                    };
                    unsafe { LLHandle::leaked(ptr, Normal) }
                }
                fn struct_ty(ctx: Self::Ctx<'a>, fields: impl Iterator<Item = Self>, packed: bool) -> Self{
                    let mut fields = fields.map(|p| p.ptr()).collect::<Vec<_>>();
                    let ptr = unsafe{
                        llvm_sys::core::LLVMStructTypeInContext(ctx.ptr(),fields.as_mut_ptr(),fields.len().try_into().unwrap(),if packed{1}else{0})
                    };
                    unsafe { LLHandle::leaked(ptr, Normal) }
                }
            }
            impl crate::ValueKind for llvm_sys::LLVMValue {
                type Val<'a, K: 'a> = crate::LLHandle<'a, K, llvm_sys::LLVMValue>;
                type Mod<'a> = crate::LLHandle<'a, Normal, llvm_sys::LLVMModule>;
                type Func<'a> = crate::LLHandle<'a, FuncTag, llvm_sys::LLVMValue>;
                type Ty<'a> = crate::LLHandle<'a, Normal, llvm_sys::LLVMType>;
                fn const_int<'a>(ty: Self::Ty<'a>, n: u64, sext: bool) -> Self::Val<'a, Normal> {
                    let ptr = ty.ptr();
                    let ptr =
                        unsafe { llvm_sys::core::LLVMConstInt(ptr, n, if sext { 1 } else { 0 }) };
                    unsafe { crate::LLHandle::leaked(ptr, Normal) }
                }
                fn function<'a, 'b, 'c, 'd: 'a + 'b + 'c>(
                    r#mod: Self::Mod<'a>,
                    name: &'b CStr,
                    ty: Self::Ty<'c>,
                ) -> Self::Func<'d> {
                    let ptr = unsafe {
                        llvm_sys::core::LLVMAddFunction(r#mod.ptr(), name.as_ptr(), ty.ptr())
                    };
                    unsafe { crate::LLHandle::leaked(ptr, FuncTag) }
                }
            }
            impl<'a> crate::Ctx<'a> for crate::LLHandle<'a, Normal, llvm_sys::LLVMContext> {}
            impl<'a> crate::Mod<'a> for crate::LLHandle<'a, Normal, llvm_sys::LLVMModule> {
                type Ctx<'b>
                    = crate::LLHandle<'b, Normal, llvm_sys::LLVMContext>
                where
                    Self: 'b;
                fn ctx<'b: 'a>(&'b self) -> Self::Ctx<'b> {
                    let ptr = self.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMGetModuleContext(ptr) };
                    unsafe { crate::LLHandle::leaked(ptr, Normal) }
                }
                fn create_mod<'b, 'c, 'd>(a: &'b CStr, ctx: &'c Self::Ctx<'d>) -> Self
                where
                    'a: 'b + 'c + 'd,
                {
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
            impl<'a> crate::Func<'a> for crate::LLHandle<'a, FuncTag, llvm_sys::LLVMValue> {}
            impl<'a> crate::BB<'a> for crate::LLHandle<'a, Normal, llvm_sys::LLVMBasicBlock> {
                type Func<'b>
                    = crate::LLHandle<'b, FuncTag, llvm_sys::LLVMValue>
                where
                    'a: 'b,
                    Self: 'b;
                fn new<'b, 'c>(f: Self::Func<'b>, name: &'c CStr) -> Self
                where
                    'a: 'b + 'c,
                {
                    let ptr = f.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMAppendBasicBlock(ptr, name.as_ptr()) };
                    unsafe { crate::LLHandle::leaked(ptr, Normal) }
                }
            }
            impl<'a> crate::Builder<'a> for crate::LLHandle<'a, Normal, llvm_sys::LLVMBuilder> {
                type BB<'b,'e,'d>
                    = crate::LLHandle<'b, Normal, llvm_sys::LLVMBasicBlock>
                where
                    Self: 'b,
                    'a: 'b, Self: 'e, Self: 'd;
                type ValKind<'d, 'b> = llvm_sys::LLVMValue  where
                Self: 'd,
                Self: 'b;
                // type InternalValShim<'b: 'a, 'd, 'e, K: 'b> = crate::LLHandle<'b,K,llvm_sys::LLVMValue> where K: Sized, K: 'b, Self: 'd, Self: 'e;
                // type Val<'b: 'a,K: 'b> = crate::LLHandle<'b,K,llvm_sys::LLVMValue> where K: Sized, K: 'b;
                type Ty<'b>
                    = crate::LLHandle<'b, Normal, llvm_sys::LLVMType>
                where
                    Self: 'b;
                type Ctx<'b>
                    = crate::LLHandle<'b, Normal, llvm_sys::LLVMContext>
                where
                    Self: 'b;
                type Mod<'b> = crate::LLHandle<'b, Normal, llvm_sys::LLVMModule> where Self: 'b;
                fn new_in_ctx(ctx: Self::Ctx<'a>) -> Self {
                    let ptr = ctx.ptr();
                    let ptr = unsafe { llvm_sys::core::LLVMCreateBuilderInContext(ptr) };
                    unsafe {
                        crate::LLHandle::from_raw_parts(
                            ptr,
                            |a, _| llvm_sys::core::LLVMDisposeBuilder(a),
                            Normal,
                        )
                    }
                }
                fn r#continue<'b, 'c>(&'b self, bb: Self::BB<'c,'a,'a>)
                where
                    'a: 'b + 'c,
                {
                    unsafe { llvm_sys::core::LLVMPositionBuilderAtEnd(self.ptr(), bb.ptr()) }
                }
                fn call<'b, 'c, 'd, 'e, 'f,'h,'i, 'g: 'a + 'b + 'c + 'd + 'e + 'f + 'h + 'i>(
                    &'b self,
                    resty: Self::Ty<'c>,
                    r#fn: <Self::ValKind<'a,'a> as ValueKind>::Val<'d, Normal>,
                    args: impl Iterator<Item = <Self::ValKind<'h,'i> as ValueKind>::Val<'e, Normal>>,
                    name: &'f CStr,
                ) -> <Self::ValKind<'_,'_> as ValueKind>::Val<'g, Normal> where 'a: 'h + 'i, Self: 'c{
                    let ptr = self.ptr();
                    let resty = resty.ptr();
                    let Fn = r#fn.ptr();
                    let mut args = args.map(|a| a.ptr()).collect::<Vec<_>>();
                    let res = unsafe {
                        llvm_sys::core::LLVMBuildCall2(
                            ptr,
                            resty,
                            Fn,
                            args.as_mut_ptr(),
                            args.len().try_into().unwrap(),
                            name.as_ptr(),
                        )
                    };
                    unsafe { crate::LLHandle::leaked(res, Normal) }
                }
                fn gep2<'b, 'c, 'd, 'e, 'f, 'h, 'i, 'g: 'a + 'b + 'c + 'd + 'e + 'f + 'h + 'i>(
                    &'b self,
                    resty: Self::Ty<'c>,
                    ptr2: <Self::ValKind<'_, '_> as ValueKind>::Val<'d, Normal>,
                    args: impl Iterator<Item = <Self::ValKind<'h, 'i> as ValueKind>::Val<'e, Normal>>,
                    name: &'f CStr,
                ) -> <Self::ValKind<'_, '_> as ValueKind>::Val<'g, Normal>
                where
                    Self: 'h + 'i{
                        let ptr = self.ptr();
                        let resty = resty.ptr();
                        let Fn = ptr2.ptr();
                        let mut args = args.map(|a| a.ptr()).collect::<Vec<_>>();
                        let res = unsafe {
                            llvm_sys::core::LLVMBuildGEP2(
                                ptr,
                                resty,
                                Fn,
                                args.as_mut_ptr(),
                                args.len().try_into().unwrap(),
                                name.as_ptr(),
                            )
                        };
                        unsafe { crate::LLHandle::leaked(res, Normal) }
                    }
                default_insts!('a @ llvm_sys);
            }
        };
    };
}

llvm_codegen_utils_version_macros::vers!({} impls);
