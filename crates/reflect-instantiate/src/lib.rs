pub use proc_macro2 as __proc_macro2;
pub use quote as __quote;

use proc_macro2::TokenStream;
use quote::ToTokens;

// export derive macro
pub use reflect_instantiate_derive::ReflectInstantiate;

/// Allows a value of this type to be reflected and instantiated at compile-time via a proc-macro.
///
/// Can be derived. See [`reflect_instantiate_derive`].
pub trait ReflectInstantiate {
    /// Reflect this value.
    fn instantiate(&self) -> TokenStream;
}

macro_rules! impl_primitive {
    ($type:ty) => {
        impl ReflectInstantiate for $type {
            fn instantiate(&self) -> TokenStream {
                quote::quote! {#self}
            }
        }
    };
}

impl_primitive!(bool);

impl_primitive!(char);
impl_primitive!(&str);
impl_primitive!(String);

impl_primitive!(f32);
impl_primitive!(f64);

impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(isize);

impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(usize);

impl<T: ToTokens> ReflectInstantiate for &T {
    fn instantiate(&self) -> TokenStream {
        quote::quote! {#self}
    }
}

impl<T: ToTokens> ReflectInstantiate for &mut T {
    fn instantiate(&self) -> TokenStream {
        quote::quote! {#self}
    }
}

impl<T: ToTokens> ReflectInstantiate for Box<T> {
    fn instantiate(&self) -> TokenStream {
        quote::quote! {#self}
    }
}

impl<T: ToTokens> ReflectInstantiate for Option<T> {
    fn instantiate(&self) -> TokenStream {
        quote::quote! {#self}
    }
}
