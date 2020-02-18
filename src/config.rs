use crate::{Apply, Diff, FieldPathMode, SerdeDiff};
use serde::{de, Serialize, Serializer};
use std::cell::Cell;

/// Configures creation of `Apply` and `Diff`
///
/// # Examples
///
/// ```rust
/// use serde_diff::{SerdeDiff, Config, FieldPathMode};
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
/// struct Test {
///     a: i32,
/// }
/// let diff = Config::new()
///     .with_field_path_mode(FieldPathMode::Index)
///     .serializable_diff(&Test { a: 3 }, &Test { a: 5 });
/// ```
pub struct Config {
    field_path_mode: FieldPathMode,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            field_path_mode: FieldPathMode::Name,
        }
    }
}

impl Config {
    /// Creates a `Config` with default values
    pub fn new() -> Self {
        <Self as Default>::default()
    }

    /// Sets the `FieldPathMode` to use when serializing a Diff
    pub fn with_field_path_mode(mut self, mode: FieldPathMode) -> Self {
        self.field_path_mode = mode;
        self
    }

    /// Create a serializable Diff, which when serialized will write the differences between the old
    /// and new value into the serializer in the form of a sequence of diff commands
    pub fn serializable_diff<'a, 'b, T: SerdeDiff + 'a + 'b>(
        self,
        old: &'a T,
        new: &'b T,
    ) -> Diff<'a, 'b, T> {
        Diff {
            old,
            new,
            field_path_mode: self.field_path_mode,
            has_changes: Cell::new(false),
        }
    }

    /// Writes the differences between the old and new value into the given serializer in the form
    /// of a sequence of diff commands
    pub fn diff<'a, 'b, S: Serializer, T: SerdeDiff + 'a + 'b>(
        self,
        serializer: S,
        old: &'a T,
        new: &'b T,
    ) -> Result<S::Ok, S::Error> {
        self.serializable_diff(old, new).serialize(serializer)
    }

    /// Create a deserializable Apply, where the given target will be changed when the resulting
    /// Apply struct is deserialized
    pub fn deserializable_apply<'a, T: SerdeDiff>(self, target: &'a mut T) -> Apply<'a, T> {
        Apply { target }
    }

    /// Applies a sequence of diff commands to the target, as read by the deserializer
    pub fn apply<'de, D, T: SerdeDiff>(
        self,
        deserializer: D,
        target: &mut T,
    ) -> Result<(), <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self.deserializable_apply(target))
    }
}
