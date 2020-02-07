use crate::{
    difference::{
        DeserWrapper, DiffCommandDeserWrapper, DiffCommandIgnoreValue, DiffCommandValue,
        DiffPathElementValue,
    },
    Config, SerdeDiff,
};
use serde::{de, Deserialize};

/// A deserializable structure that will apply a sequence of diff commands to the target
///
/// # Examples
///
/// ```rust
/// use serde_diff::{SerdeDiff, Diff, Apply};
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
/// struct Test {
///     a: i32,
/// }
/// let diff = Diff::serializable(&Test { a: 3 }, &Test { a: 5 });
/// let msgpack_data = rmp_serde::to_vec_named(&diff).expect("failed to serialize diff");
/// let mut deserializer = rmp_serde::Deserializer::new(msgpack_data.as_slice());
/// let mut target = Test { a: 4 };
/// Apply::apply(&mut deserializer, &mut target).expect("failed when deserializing diff");
/// ```
pub struct Apply<'a, T: SerdeDiff> {
    pub(crate) target: &'a mut T,
}

impl<'a, 'de, T: SerdeDiff> Apply<'a, T> {
    /// Create a deserializable apply, where the given target will be changed when the resulting
    /// Apply struct is deserialized
    pub fn deserializable(target: &'a mut T) -> Self {
        Config::default().deserializable_apply(target)
    }

    /// Applies a sequence of diff commands to the target, as read by the deserializer
    pub fn apply<D>(
        deserializer: D,
        target: &mut T,
    ) -> Result<(), <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        Config::default().apply(deserializer, target)
    }
}

impl<'a, 'de, T: SerdeDiff> de::DeserializeSeed<'de> for Apply<'a, T> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'a, 'de, T: SerdeDiff> de::Visitor<'de> for Apply<'a, T> {
    type Value = ();
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a sequence containing DiffCommands")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut ctx = ApplyContext {};
        self.target.apply(&mut seq, &mut ctx)?;
        Ok(())
    }
}

/// Used during an apply operation for transient data used during the apply
#[doc(hidden)]
pub struct ApplyContext {}

impl ApplyContext {
    /// Returns the next element if it is a path. If it is a Value or Exit, it returns None.
    pub fn next_path_element<'de, A>(
        &mut self,
        seq: &mut A,
    ) -> Result<Option<DiffPathElementValue<'de>>, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        use DiffCommandValue::*;
        let element = match seq.next_element_seed(DiffCommandIgnoreValue {})? {
            Some(Enter(element)) => Ok(Some(element)),
            Some(AddKey(_)) | Some(EnterKey(_)) | Some(RemoveKey(_)) => {
                //self.skip_value(seq);
                Ok(None)
            }
            Some(Value(_)) | Some(Remove(_)) => panic!("unexpected DiffCommand Value or Remove"),
            Some(Exit) | Some(Nothing) | Some(DeserializedValue) | None => Ok(None),
        };
        element
    }
    /// To be called after next_path_element returns a path, but the path is not recognized.
    pub fn skip_value<'de, A>(
        &mut self,
        seq: &mut A,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        self.skip_value_internal(seq, 1)
    }
    fn skip_value_internal<'de, A>(
        &mut self,
        seq: &mut A,
        mut depth: i32,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // this tries to skip the value without knowing the type - not possible for some formats..
        while let Some(cmd) = seq.next_element_seed(DiffCommandIgnoreValue {})? {
            match cmd {
                DiffCommandValue::Enter(_)
                | DiffCommandValue::AddKey(_)
                | DiffCommandValue::EnterKey(_) => depth += 1,
                DiffCommandValue::Exit => depth -= 1,
                DiffCommandValue::Value(_) | DiffCommandValue::Remove(_) => depth -= 1, // ignore value, but reduce depth, as it is an implicit Exit
                DiffCommandValue::RemoveKey(_) => {}
                DiffCommandValue::Nothing | DiffCommandValue::DeserializedValue => {
                    panic!("should never serialize cmd Nothing or DeserializedValue")
                }
            }
            if depth == 0 {
                break;
            }
        }
        if depth != 0 {
            panic!("mismatched DiffCommand::Enter/Exit ")
        }
        Ok(())
    }
    /// Attempts to deserialize a value
    pub fn read_value<'de, A, T: for<'c> Deserialize<'c>>(
        &mut self,
        seq: &mut A,
        val: &mut T,
    ) -> Result<bool, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // The visitor for DiffCommandDeserWrapper handles enum cases and returns
        // a command if the next element was not a Value
        let cmd = seq.next_element_seed::<DiffCommandDeserWrapper<T>>(DiffCommandDeserWrapper {
            val_wrapper: DeserWrapper { val },
        })?;
        match cmd {
            Some(DiffCommandValue::DeserializedValue) => return Ok(true),
            Some(DiffCommandValue::Enter(_)) => {
                self.skip_value_internal(seq, 1)?;
            }
            Some(DiffCommandValue::Exit) => panic!("unexpected Exit command"),
            _ => {}
        }

        Ok(false)
    }
    /// Returns the next command in the stream. Make sure you know what you're doing!
    pub fn read_next_command<'de, A, T: for<'c> Deserialize<'c>>(
        &mut self,
        seq: &mut A,
    ) -> Result<Option<DiffCommandValue<'de, T>>, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // The visitor for DiffCommandDeserWrapper handles enum cases and returns
        // a command if the next element was not a Value
        let cmd = seq.next_element::<DiffCommandValue<'de, T>>()?;
        Ok(match cmd {
            cmd @ Some(DiffCommandValue::Remove(_))
            | cmd @ Some(DiffCommandValue::Value(_))
            | cmd @ Some(DiffCommandValue::Enter(_))
            | cmd @ Some(DiffCommandValue::AddKey(_))
            | cmd @ Some(DiffCommandValue::EnterKey(_))
            | cmd @ Some(DiffCommandValue::RemoveKey(_))
            | cmd @ Some(DiffCommandValue::Exit) => cmd,
            _ => None,
        })
    }
}
