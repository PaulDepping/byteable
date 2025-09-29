use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(Byteable)]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: DeriveInput = parse_macro_input!(input);

    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics Byteable for #ident #type_generics #where_clause {
            type ByteArray = [u8; std::mem::size_of::<Self>()];
            fn as_bytearray(self) -> Self::ByteArray {
                unsafe { std::mem::transmute(self) }
            }
            fn from_bytearray(ba: Self::ByteArray) -> Self {
                unsafe { std::mem::transmute(ba) }
            }
        }
    }
    .into()
}
