use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::{DeriveInput, Ident, parse_macro_input};

#[proc_macro_derive(UnsafeByteable)]
pub fn byteable_derive_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let found_crate = crate_name("byteable").expect("my-crate is present in `Cargo.toml`");
    let byteable = match found_crate {
        FoundCrate::Itself => quote!(crate::Byteable),
        FoundCrate::Name(name) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!( #ident::Byteable )
        }
    };

    let input: DeriveInput = parse_macro_input!(input);

    let ident = &input.ident;

    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #byteable for #ident #type_generics #where_clause {
            type ByteArray = [u8; ::std::mem::size_of::<Self>()];
            fn as_byte_array(self) -> Self::ByteArray {
                unsafe { ::std::mem::transmute(self) }
            }

            fn from_byte_array(byte_array: Self::ByteArray) -> Self {
                unsafe { ::std::mem::transmute(byte_array) }
            }
        }
    }
    .into()
}
