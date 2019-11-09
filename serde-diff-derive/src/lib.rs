extern crate proc_macro;

mod serde_diff;

#[proc_macro_derive(SerdeDiff, attributes(serde_diff))]
pub fn serde_diff_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    serde_diff::macro_derive(input)
}
