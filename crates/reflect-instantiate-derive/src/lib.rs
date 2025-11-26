use proc_macro::TokenStream as TokenStreamV1;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Ident, parse_macro_input};

#[proc_macro_derive(ReflectInstantiate)]
pub fn derive_reflect_instantiate(input: TokenStreamV1) -> TokenStreamV1 {
    let input = parse_macro_input!(input as DeriveInput);

    convert(input).into()
}

fn convert(input: DeriveInput) -> TokenStream {
    let type_ident = input.ident;

    // TODO: support other data types
    let fields: Vec<Ident> = if let Data::Struct(data) = input.data {
        data.fields
            .iter()
            .map(|field| {
                field
                    .ident
                    .clone()
                    .expect("only named fields are supported")
            })
            .collect()
    } else {
        panic!("only structs are supported");
    };

    let instantiate = instantiate(type_ident.clone(), fields.clone());

    quote! {
        impl reflect_instantiate::ReflectInstantiate for #type_ident {
            fn instantiate(&self) -> reflect_instantiate::__proc_macro2::TokenStream {
                #instantiate
            }
        }
    }
}

fn instantiate(type_ident: Ident, field_idents: Vec<Ident>) -> TokenStream {
    // recurse on each field of struct
    let mut recurse = TokenStream::new();
    for ident in field_idents.iter() {
        recurse.extend(quote! {
            let #ident = self.#ident.instantiate();
        });
    }

    // instantiate the value
    let mut instance_inner = TokenStream::new();
    for ident in field_idents.iter() {
        instance_inner.extend(quote! {
            #ident: # #ident,
        });
    }

    let instance = quote! {
        #type_ident {
            #instance_inner
        }
    };

    quote! {
        #recurse
        reflect_instantiate::__quote::quote!{ #instance }
    }
}
