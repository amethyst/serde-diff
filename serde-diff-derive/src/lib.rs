extern crate proc_macro;

mod serde_diff;

/// # Examples
///
/// Minimal example of implementing diff support for a struct
/// ```rust
/// use serde_diff::SerdeDiff;
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize)]
/// struct MySimpleStruct {
///    val: u32,
/// }
/// ```
///
/// Example of an opaque (non-recursive diff) implementation of SerdeDiff using `#[serde_diff(opaque)]` on the struct.
/// Field types are not required to implement SerdeDiff in this case, only Serialize + Deserialize + PartialEq.
/// ```rust
/// use serde_diff::SerdeDiff;
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
/// #[serde_diff(opaque)]
/// struct OpaqueTest(i32);
/// ```
///
/// Example of a struct with an opaque field using `#[serde_diff(opaque)]` on a field.
/// ```rust
/// use serde_diff::SerdeDiff;
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Clone, Serialize, Deserialize, Debug)]
/// struct MyInnerStruct {
///     #[serde_diff(opaque)]
///     heap: std::collections::HashSet<i32>,
/// }
/// ```
#[proc_macro_derive(SerdeDiff, attributes(serde_diff))]
pub fn serde_diff_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    serde_diff::macro_derive(input)
}
