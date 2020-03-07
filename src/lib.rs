#![warn(missing_docs)]
//! A small helper that can
//! 1. Serialize the fields that differ between two structs of the same type
//! 2. Apply previously serialized field differences to other structs
//!
//! The SerdeDiff trait impl can serialize field paths recursively,
//! greatly reducing the amount of data that needs to be serialized
//! when only a small part of a struct has changed.

#[cfg(test)]
mod tests;

#[doc(hidden)]
pub use serde as _serde;
use serde::{de, ser::SerializeSeq};
pub use serde_diff_derive::SerdeDiff;

#[doc(hidden)]
pub(crate) mod apply;
pub(crate) mod config;
pub(crate) mod counting_serializer;
#[doc(hidden)]
pub(crate) mod difference;
pub(crate) mod implementation;

pub use apply::Apply;
pub use config::Config;
pub use difference::Diff;

// Used by the proc_macro
pub use apply::ApplyContext;
pub use difference::DiffContext;
pub use difference::DiffPathElementValue;

// NEXT STEPS:
// - Decouple from serde_json as much as possible. We might need to use a "stream" format with
//   well-defined data order to be able to use serde Deserializer trait. DONE
// - Make all fields work again. DONE
// - Make it work via proc macro. DONE
// - Blanket impl or impl-via-macro common std types (i.e f32, i32, String). DONE
// - Handle containers. DONE
// - Ignore type mismatches instead of propagating the error. IMPOSSIBLE??
//
// A problem occurs when encoding the command stream for bincode:
// We need to know the size of the list before we start serializing.
// To do so, we need to implement the serde::ser::Serializer trait and
// make the implementation only count up every time an element is serialized, doing nothing else.
// This is implemented as CountingSerializer

/// Anything diffable implements this trait
pub trait SerdeDiff {
    /// Recursively walk the struct, invoking serialize_element on each member if the element is
    /// different. Returns true if any changes exist, otherwise false. After this call, the
    /// DiffContext will contain the data that has changed.
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut difference::DiffContext<'a, S>,
        other: &Self,
    ) -> Result<bool, S::Error>;

    /// Applies the diff to the struct. Returns true if the struct was changed, otherwise false.
    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut apply::ApplyContext,
    ) -> Result<bool, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>;
}

/// Configures how to serialize field identifiers
#[derive(Copy, Clone)]
pub enum FieldPathMode {
    /// Use the field's string name as its identifier
    Name,
    /// Use the field's index in the struct as its identifier
    Index,
}

pub(crate) enum ElementStackEntry<'a, S: SerializeSeq> {
    PathElement(difference::DiffPathElementValue<'a>),
    Closure(&'a dyn Fn(&mut S) -> Result<(), S::Error>),
}
