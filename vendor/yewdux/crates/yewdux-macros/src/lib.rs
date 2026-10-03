use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod store;

#[proc_macro_derive(Store, attributes(store))]
pub fn store(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    store::derive(input).into()
}
