use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::{replace, take, MaybeUninit};
use std::sync::{Arc, Mutex};
use std::thread::LocalKey;
use std::thread_local;

use nonempty::NonEmpty;
use typenum::Same;
mod private {
    pub trait Sealed {}
}

pub trait Ctx<'a>: Clone + private::Sealed + 'a {}
pub trait Mod<'a>: Clone + private::Sealed + 'a {
    type Ctx<'b>: Ctx<'b>
    where
        Self: 'b;
    fn ctx<'b: 'a>(&'b self) -> Self::Ctx<'b>;
    fn create_mod<'b, 'c, 'd>(a: &'b CStr, ctx: &'c Self::Ctx<'d>) -> Self
    where
        'a: 'b + 'c + 'd;
}
pub trait Value<'a>: Clone + private::Sealed + 'a {
    type Tag: 'a;
    type Kind: for<'b> ValueKind<Val<'a, Self::Tag> = Self, Mod<'b> = Self::Mod<'b>>;
    type Mod<'b>: Mod<'b>;
    fn r#mod<'b: 'a>(&'b self) -> Self::Mod<'b>;
}
pub trait ValueKind: private::Sealed {
    type Mod<'a>: Mod<'a>;
    type Val<'a, K: 'a>: for<'b> Value<'a, Tag = K, Kind = Self, Mod<'b> = Self::Mod<'b>>
    where
        K: 'a;
    type Func<'a>: for<'b> Func<'a, Kind = Self, Mod<'b> = Self::Mod<'b>>;
    type Ty<'a>: Ty<'a>;
    fn const_int<'a>(ty: Self::Ty<'a>, n: u64, sext: bool) -> Self::Val<'a, Normal>;
    fn function<'a, 'b, 'c, 'd: 'a + 'b + 'c>(
        r#mod: Self::Mod<'a>,
        name: &'b CStr,
        ty: Self::Ty<'c>,
    ) -> Self::Func<'d>;
}
pub trait Func<'a>: Clone + private::Sealed + Value<'a, Tag = FuncTag> + 'a {}
pub trait BB<'a>: Clone + private::Sealed + 'a {
    type Func<'b>: Func<'b>
    where
        'a: 'b,
        Self: 'b;
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
    (($l2:lifetime)@ $i:ident ($(($l:lifetime) @ $e:ident : $t:ty as |$v:ident|$b:expr),*) =>  $($llvm:ident )? => $stuff:tt) => {
        paste::paste!{
            #[allow(unreachable_code,unused_variables)]
            fn $i<'b,$($l),*,'res:  $($l + )* 'b>(&'b self, $($e: $t),*) -> <Self::ValKind<'a,'a> as ValueKind>::Val<'res,Normal> where $($l2 : $l),*{

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
    (($l2:lifetime)@{[$($t0:tt)*], $([$($t:tt)*],)*} => $(<$llvm:ident>)?) => {
        inst!(($l2)@[$($t0)*] => $($llvm)?);
        insts!(($l2)@{$([$($t)*],)*} => $(<$llvm>)?);
    };
    (($l2:lifetime)@{} => $(<$llvm:ident>)?) => {

    };
}
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[non_exhaustive]
pub enum ICmp {
    Eq,
    Lt,
    Lts,
}
macro_rules! default_insts {
    ($l2:lifetime @ $($llvm:ident)?) => {
        insts!(($l2) @ {
            [Alloca (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
            [Load2 (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
            [StructGEP2 (('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr(), ('idx) @ idx: &'idx u32 as |x|*x, ('name) @ name : &'name CStr as |x|x.as_ptr())],
            [Store (('val) @ value: <Self::ValKind<'a,'a> as ValueKind>::Val<'val,Normal> as |x|x.ptr(), ('ptr) @ pointer: <Self::ValKind<'a,'a> as ValueKind>::Val<'ptr,Normal> as |x|x.ptr())],
            [Add (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
            [And (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
            [Neg (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(),  ('name) @ name : &'name CStr as |x|x.as_ptr())],
      [  Not (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
      [ TruncOrBitCast (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(),('ty) @ ty: Self::Ty<'ty> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
      [Mul (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
      [Or (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
     [ Sub (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
     [Xor (('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],
         [ICmp (('op) @ op: crate::ICmp as |a|a.into(),('lhs) @ lhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'lhs,Normal> as |x|x.ptr(), ('rhs) @ rhs: <Self::ValKind<'a,'a> as ValueKind>::Val<'rhs,Normal> as |x|x.ptr(), ('name) @ name : &'name CStr as |x|x.as_ptr())],

     [ Br (('dest) @ dest: Self::BB<'dest,'a,'a> as |x|x.ptr())],
     [CondBr (('cond) @ r#if: <Self::ValKind<'a,'a> as ValueKind>::Val<'cond,Normal> as |x|x.ptr(), ('then) @ then: Self::BB<'then,'a,'a> as |x|x.ptr(),('e) @ r#else: Self::BB<'e,'a,'a> as |x|x.ptr())],
        } => $(<$llvm>)?);
    };
}
pub trait Ty<'a>: Clone + private::Sealed + 'a {
    type Ctx<'b>: Ctx<'b>
    where
        Self: 'b;
    fn int_ty(ctx: Self::Ctx<'a>, size: u32) -> Self;
    fn ptr_ty(ctx: Self::Ctx<'a>, address_space: u32) -> Self;
    fn fun_ty(self, params: impl Iterator<Item = Self>) -> Self;
}
pub trait Builder<'a>: Clone + private::Sealed + 'a {
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
    default_insts!('a @ );
}
pub struct LLHandle<'a, K, T>(Arc<LLShim<'a, K, T>>);
impl<'a, K, T> Clone for LLHandle<'a, K, T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<'a, K, T> LLHandle<'a, K, T> {
    pub unsafe fn from_raw_parts(ptr: *mut T, dropper: fn(*mut T, K), key: K) -> Self {
        LLHandle(Arc::new(LLShim {
            val: ptr,
            dropper: dropper,
            key: MaybeUninit::new(key),
            phantom: PhantomData,
        }))
    }
    pub unsafe fn leaked(ptr: *mut T, key: K) -> Self {
        unsafe { Self::from_raw_parts(ptr, |_, _| {}, key) }
    }
    pub fn ptr(&self) -> *mut T {
        return self.0.val;
    }
    pub fn key(&self) -> &K {
        return unsafe { self.0.key.assume_init_ref() };
    }
}
pub struct LLShim<'a, K, T> {
    val: *mut T,
    key: MaybeUninit<K>,
    dropper: fn(*mut T, K),
    phantom: PhantomData<&'a T>,
}
impl<'a, K, T> Drop for LLShim<'a, K, T> {
    fn drop(&mut self) {
        (self.dropper)(self.val, unsafe {
            replace(&mut self.key, MaybeUninit::uninit()).assume_init()
        })
    }
}
macro_rules! seal {
    ($(<$($generics:lifetime),*> => $t:ty),* $(,)?) => {
        $(impl<$($generics),*> private::Sealed for $t{})*
    };
}
pub struct Normal;
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
                default_insts!('a @ llvm_sys);
            }
        };
    };
}

llvm_codegen_utils_version_macros::vers!({} impls);
