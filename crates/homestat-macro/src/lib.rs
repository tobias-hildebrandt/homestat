use std::fs::File;

use homestat_build::Cyw43439Regions;
use proc_macro::TokenStream as TokenStreamV1;
use quote::quote;
use reflect_instantiate::ReflectInstantiate;
use syn::LitStr;
use syn::parse_macro_input;

/// Include a [`Cyw43439Regions`] decoded from a JSON file.
// TODO: allow concatenating env!() vars
#[proc_macro]
pub fn include_cyw_regions(input: TokenStreamV1) -> TokenStreamV1 {
    let filename = parse_macro_input!(input as LitStr).value();

    let file =
        File::open(&filename).unwrap_or_else(|e| panic!("unable to open file {}: {e:?}", filename));
    let regions = Cyw43439Regions::read_json(file).expect("unable to parse file");

    let instance = regions.instantiate();

    // we can't know the fully-qualified type names in the derive macro, so we just import them here
    let full = quote! {
        {
            use homestat_build::{Cyw43439Regions, FlashRegion};

            #instance
        }
    };

    full.into()
}
