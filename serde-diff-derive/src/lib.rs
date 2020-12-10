extern crate proc_macro;

mod serde_diff;

/// # Examples
///
/// Minimal example of implementing diff support for a struct
/// ```rust
/// use serde_diff::SerdeDiff;
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff)]
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
/// #[derive(SerdeDiff)]
/// struct MyInnerStruct {
///     #[serde_diff(opaque)]
///     heap: std::collections::HashSet<i32>,
/// }
/// ```
///
/// Example of diffing a target struct `MySimpleStruct` that is being used for serialization instead
/// of the struct `MyComplexStruct` itself. Useful for cases where derived data is present at
/// runtime, but not wanted in the serialized form.
/// ```rust
/// use serde_diff::SerdeDiff;
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize, Clone)]
/// #[serde(from = "MySimpleStruct", into = "MySimpleStruct")]
/// #[serde_diff(target = "MySimpleStruct")]
/// struct MyComplexStruct {
///    val: u32,
///    derived_val: String,
/// }
///
/// #[derive(SerdeDiff, Serialize, Deserialize, Default)]
/// #[serde(rename = "MyComplexStruct", default)]
/// struct MySimpleStruct {
///    val: u32,
/// }
///
/// impl From<MySimpleStruct> for MyComplexStruct {
///    fn from(my_simple_struct: MySimpleStruct) -> Self {
///        MyComplexStruct {
///            val: my_simple_struct.val,
///            derived_val: my_simple_struct.val.to_string(),
///        }
///    }
/// }
///
/// impl Into<MySimpleStruct> for MyComplexStruct {
///     fn into(self) -> MySimpleStruct {
///         MySimpleStruct {
///             val: self.val,
///         }
///     }
/// }
/// ```
#[proc_macro_derive(SerdeDiff, attributes(serde_diff))]
pub fn serde_diff_macro_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    serde_diff::macro_derive(input)
}
