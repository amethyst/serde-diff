extern crate proc_macro;

mod serde_diffable;

#[proc_macro_derive(SerdeDiffable, attributes(serde_diffable))]
pub fn serde_diffable_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    serde_diffable::diffable_macro_derive(input)
}
