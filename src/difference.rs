use crate::{
    apply::ApplyContext, counting_serializer::CountingSerializer, Config, ElementStackEntry,
    FieldPathMode, SerdeDiff,
};
use serde::{de, ser::SerializeSeq, Deserialize, Serialize, Serializer};
use std::{borrow::Cow, cell::Cell};

/// Used during a diff operation for transient data used during the diff
#[doc(hidden)]
pub struct DiffContext<'a, S: SerializeSeq> {
    /// As we descend into fields recursively, the field names (or other "placement" indicators like
    /// array indexes) are pushed and popped to/from this stack
    element_stack: Option<Vec<ElementStackEntry<'a, S>>>,
    /// Reference to the serializer used to save the data
    serializer: &'a mut S,
    /// some commands are implicit Exit to save space, so we set a flag to avoid writing the next Exit
    implicit_exit_written: bool,
    /// When pushing field path elements, sometimes we need to narrow the lifetime.
    /// parent_element_stack contains a reference to the parent DiffContext's element_stack.
    parent_element_stack: Option<&'a mut Option<Vec<ElementStackEntry<'a, S>>>>,
    /// Contains the minimum index in the element stack at which this context has pushed elements.
    /// When the context is dropped, we have to make sure we have dropped all elements
    /// >= index before we can pass the element stack back to the parent.
    /// This is to ensure the safety invariant that a sub-context's (a `reborrow`ed context)
    /// pushed elements cannot live longer than the sub-context itself.
    element_stack_start: usize,
    /// Mode for serializing field paths
    field_path_mode: FieldPathMode,
    /// Set to true if any change is detected
    has_changes: bool,
}

impl<'a, S: SerializeSeq> Drop for DiffContext<'a, S> {
    fn drop(&mut self) {
        if let Some(parent) = self.parent_element_stack.take() {
            if let Some(mut stack) = self.element_stack.take() {
                if self.element_stack_start < stack.len() {
                    stack.drain(self.element_stack_start..);
                }
                parent.replace(stack);
            }
        }
    }
}

#[doc(hidden)]
impl<'a, S: SerializeSeq> DiffContext<'a, S> {
    /// Mode for serializing field paths
    pub fn field_path_mode(&self) -> FieldPathMode {
        self.field_path_mode
    }

    /// True if a change operation has been written
    pub fn has_changes(&self) -> bool {
        self.has_changes
    }

    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_path_element
    /// is called. See `pop_path_element`
    pub fn push_field(&mut self, field_name: &'static str) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(DiffPathElementValue::Field(
                Cow::Borrowed(field_name),
            )));
    }

    pub fn push_variant(&mut self, variant_name: &'static str) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(
                DiffPathElementValue::EnumVariant(Cow::Borrowed(variant_name)),
            ));
    }

    pub fn push_full_variant(&mut self) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(
                DiffPathElementValue::FullEnumVariant,
            ));
    }

    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_path_element
    /// is called. See `pop_path_element`
    pub fn push_field_index(&mut self, field_idx: u16) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(
                DiffPathElementValue::FieldIndex(field_idx),
            ));
    }

    /// Called when we visit an element within an indexed collection
    pub fn push_collection_index(&mut self, idx: usize) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(
                DiffPathElementValue::CollectionIndex(idx),
            ));
    }
    /// Called when we visit an element within a collection that is new
    pub fn push_collection_add(&mut self) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::PathElement(
                DiffPathElementValue::AddToCollection,
            ));
    }

    pub fn push_field_element(&mut self, f: &'a dyn Fn(&mut S) -> Result<(), S::Error>) {
        self.element_stack
            .as_mut()
            .unwrap()
            .push(ElementStackEntry::Closure(f));
    }

    /// Called when we finish visiting an element. See `push_field` for details
    pub fn pop_path_element(&mut self) -> Result<(), S::Error> {
        let element_stack = self.element_stack.as_mut().unwrap();
        if element_stack.is_empty() {
            // if we don't have any buffered elements, we just write Exit command directly to the serializer
            // if we've just written a field, skip the Exit
            if !self.implicit_exit_written {
                let cmd = DiffCommandRef::<()>::Exit;
                self.serializer.serialize_element(&cmd)
            } else {
                self.implicit_exit_written = false;
                Ok(())
            }
        } else {
            element_stack.pop();
            self.element_stack_start = std::cmp::min(element_stack.len(), self.element_stack_start);
            Ok(())
        }
    }

    /// Stores a value for an element that has previously been pushed using push_field or similar.
    pub fn save_value<T: Serialize>(&mut self, value: &T) -> Result<(), S::Error> {
        self.save_command(&DiffCommandRef::Value(value), true, true)
    }

    /// Write exit command
    pub fn save_exit(&mut self) -> Result<(), S::Error> {
        self.save_command::<()>(&DiffCommandRef::Exit, true, false)
    }

    /// Stores an arbitrary DiffCommand to be handled by the type.
    /// Any custom sequence of DiffCommands must be followed by Exit.
    pub fn save_command<'b, T: Serialize>(
        &mut self,
        value: &DiffCommandRef<'b, T>,
        implicit_exit: bool,
        is_change: bool,
    ) -> Result<(), S::Error> {
        let element_stack = self.element_stack.as_mut().unwrap();
        if !element_stack.is_empty() {
            // flush buffered elements as Enter* commands
            for element in element_stack.drain(0..element_stack.len()) {
                match element {
                    ElementStackEntry::PathElement(element) => self
                        .serializer
                        .serialize_element(&DiffCommandRef::<()>::Enter(element))?,
                    ElementStackEntry::Closure(closure) => (closure)(&mut self.serializer)?,
                };
            }
            self.element_stack_start = 0;
        }
        self.has_changes |= is_change;
        self.implicit_exit_written = implicit_exit;
        self.serializer.serialize_element(value)
    }

    pub fn reborrow<'c, 'd: 'c>(&'d mut self) -> DiffContext<'c, S>
    where
        'a: 'c,
        'a: 'd,
    {
        let element_stack = self.element_stack.take();
        // Some background on this then..
        // HashMaps need to be able to serialize any T as keys for EnterKey(T).
        // The usual approach to Enter* commands is to push element paths to the element stack,
        // then flush the stack into the serialized stream when we encounter a value that has changed.
        // For any T, we need to push a type-erased closure that contains a reference to something that
        // might life on the stack. This is why reborrow() exists - to create a smaller scoped lifetime
        // that can be used to push such closures that live on the stack.
        // The following transmute changes the lifetime constraints on the elements in the Vec to be
        // limited to the lifetime of the newly created context. The ownership of the Vec is then moved
        // to the parent context.
        // Safety invariant:
        // In Drop, we have to make sure that any elements that could have been created by this sub-context
        // are removed from the Vec before passing the ownership back to the parent.

        let element_stack_ref = unsafe {
            std::mem::transmute::<
                &'d mut Option<Vec<ElementStackEntry<'a, S>>>,
                &'c mut Option<Vec<ElementStackEntry<'c, S>>>,
            >(&mut self.element_stack)
        };

        DiffContext {
            element_stack_start: element_stack.as_ref().unwrap().len(),
            element_stack,
            parent_element_stack: Some(element_stack_ref),
            serializer: &mut *self.serializer,
            implicit_exit_written: self.implicit_exit_written,
            field_path_mode: self.field_path_mode,
            has_changes: false,
        }
    }
}

/// A serializable structure that will produce a sequence of diff commands when serialized.
/// You can pass this to a serializer, or use the convenience method `diff`
/// to pass your serializer along with old/new values to use when serializing the diff.
///
/// # Examples
///
/// ```rust
/// use serde_diff::{SerdeDiff, Diff};
/// use serde::{Serialize, Deserialize};
/// #[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
/// struct Test {
///     a: i32,
/// }
/// let diff = Diff::serializable(&Test { a: 3 }, &Test { a: 5 });
/// ```
pub struct Diff<'a, 'b, T> {
    pub(crate) old: &'a T,
    pub(crate) new: &'b T,
    pub(crate) field_path_mode: FieldPathMode,

    // This is a cell to provide interior mutability
    pub(crate) has_changes: Cell<bool>,
}

impl<'a, 'b, T: SerdeDiff + 'a + 'b> Diff<'a, 'b, T> {
    /// Create a serializable Diff, which when serialized will write the differences between the old
    /// and new value into the serializer in the form of a sequence of diff commands
    pub fn serializable(old: &'a T, new: &'b T) -> Self {
        Config::default().serializable_diff(old, new)
    }

    /// Writes the differences between the old and new value into the given serializer in the form
    /// of a sequence of diff commands
    pub fn diff<S: Serializer>(serializer: S, old: &'a T, new: &'b T) -> Result<S::Ok, S::Error> {
        Config::default().diff(serializer, old, new)
    }

    /// True if a change was detected during the diff
    pub fn has_changes(&self) -> bool {
        self.has_changes.get()
    }
}

impl<'a, 'b, T: SerdeDiff> Serialize for Diff<'a, 'b, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.has_changes.set(false);

        // Count the number of elements
        // This may only be needed for certain serializers like bincode,
        // so we assume that it's only required if the serializer format is not human readable.
        let num_elements = if !serializer.is_human_readable() {
            let mut serializer = CountingSerializer { num_elements: 0 };
            let mut seq = serializer.serialize_seq(None).unwrap();
            {
                let mut ctx = DiffContext {
                    element_stack_start: 0,
                    element_stack: Some(Vec::new()),
                    serializer: &mut seq,
                    implicit_exit_written: false,
                    parent_element_stack: None,
                    field_path_mode: self.field_path_mode,
                    has_changes: false,
                };
                self.old.diff(&mut ctx, &self.new).unwrap();
            }
            seq.end().unwrap();
            Some(serializer.num_elements)
        } else {
            None
        };

        // Setup the context, starting a sequence on the serializer
        let mut seq = serializer.serialize_seq(num_elements)?;
        {
            let mut ctx = DiffContext {
                element_stack_start: 0,
                element_stack: Some(Vec::new()),
                serializer: &mut seq,
                implicit_exit_written: false,
                parent_element_stack: None,
                field_path_mode: self.field_path_mode,
                has_changes: false,
            };

            // Do the actual comparison, writing diff commands (see DiffCommandRef, DiffCommandValue)
            // into the sequence
            self.old.diff(&mut ctx, &self.new)?;
            self.has_changes.set(ctx.has_changes);
        }

        // End the sequence on the serializer
        Ok(seq.end()?)
    }
}

pub(crate) struct DeserWrapper<'a, T> {
    pub(crate) val: &'a mut T,
}

pub(crate) struct DiffCommandDeserWrapper<'a, T> {
    pub(crate) val_wrapper: DeserWrapper<'a, T>,
}

// This monstrosity is based off the output of the derive macro for DiffCommand.
// The justification for this is that we want to use Deserialize::deserialize_in_place
// for DiffCommand::Value in order to support zero-copy deserialization of T.
// This is achieved by passing &mut T through the DiffCommandDeserWrapper, which parses the enum
// to the DeserWrapper which calls Deserialize::deserialize_in_place.
pub(crate) enum DiffCommandField {
    Enter,
    Value,
    Remove,
    AddKey,
    EnterKey,
    RemoveKey,
    Exit,
}

pub(crate) struct DiffCommandFieldVisitor;

const VARIANTS: &'static [&'static str] = &[
    "Enter",
    "Value",
    "Remove",
    "AddKey",
    "EnterKey",
    "RemoveKey",
    "Exit",
];

impl<'de> de::Visitor<'de> for DiffCommandFieldVisitor {
    type Value = DiffCommandField;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Formatter::write_str(formatter, "variant identifier")
    }
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            0u64 => Ok(DiffCommandField::Enter),
            1u64 => Ok(DiffCommandField::Value),
            2u64 => Ok(DiffCommandField::Remove),
            3u64 => Ok(DiffCommandField::AddKey),
            4u64 => Ok(DiffCommandField::EnterKey),
            5u64 => Ok(DiffCommandField::RemoveKey),
            6u64 => Ok(DiffCommandField::Exit),
            _ => Err(de::Error::invalid_value(
                de::Unexpected::Unsigned(value),
                &"variant index 0 <= i < 7",
            )),
        }
    }
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            "Enter" => Ok(DiffCommandField::Enter),
            "Value" => Ok(DiffCommandField::Value),
            "Remove" => Ok(DiffCommandField::Remove),
            "AddKey" => Ok(DiffCommandField::AddKey),
            "EnterKey" => Ok(DiffCommandField::EnterKey),
            "RemoveKey" => Ok(DiffCommandField::RemoveKey),
            "Exit" => Ok(DiffCommandField::Exit),
            _ => Err(de::Error::unknown_variant(value, VARIANTS)),
        }
    }
    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        match value {
            b"Enter" => Ok(DiffCommandField::Enter),
            b"Value" => Ok(DiffCommandField::Value),
            b"Remove" => Ok(DiffCommandField::Remove),
            b"AddKey" => Ok(DiffCommandField::AddKey),
            b"EnterKey" => Ok(DiffCommandField::EnterKey),
            b"RemoveKey" => Ok(DiffCommandField::RemoveKey),
            b"Exit" => Ok(DiffCommandField::Exit),
            _ => {
                let value = &String::from_utf8_lossy(value);
                Err(de::Error::unknown_variant(value, VARIANTS))
            }
        }
    }
}

impl<'de> Deserialize<'de> for DiffCommandField {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        de::Deserializer::deserialize_identifier(deserializer, DiffCommandFieldVisitor)
    }
}

impl<'a, 'de, T> de::DeserializeSeed<'de> for DiffCommandDeserWrapper<'a, T>
where
    T: de::Deserialize<'de>,
{
    type Value = DiffCommandValue<'de, T>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor<'de, 'a, T>
        where
            T: de::Deserialize<'de>,
        {
            seed: DeserWrapper<'a, T>,
            lifetime: std::marker::PhantomData<&'de ()>,
        }
        impl<'de, 'a, T> de::Visitor<'de> for Visitor<'de, 'a, T>
        where
            T: de::Deserialize<'de>,
        {
            type Value = DiffCommandValue<'de, T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Formatter::write_str(formatter, "enum DiffCommandValueTest")
            }
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                match de::EnumAccess::variant(data)? {
                    (DiffCommandField::Enter, variant) => {
                        let enter =
                            de::VariantAccess::newtype_variant::<DiffPathElementValue>(variant)?;
                        Ok(DiffCommandValue::Enter(enter))
                    }
                    (DiffCommandField::Value, variant)
                    | (DiffCommandField::AddKey, variant)
                    | (DiffCommandField::EnterKey, variant)
                    | (DiffCommandField::RemoveKey, variant) => {
                        de::VariantAccess::newtype_variant_seed::<DeserWrapper<T>>(
                            variant, self.seed,
                        )?;
                        Ok(DiffCommandValue::DeserializedValue)
                    }
                    (DiffCommandField::Remove, variant) => {
                        let num_elements = de::VariantAccess::newtype_variant::<usize>(variant)?;
                        Ok(DiffCommandValue::Remove(num_elements))
                    }
                    (DiffCommandField::Exit, variant) => {
                        de::VariantAccess::unit_variant(variant)?;
                        Ok(DiffCommandValue::Exit)
                    }
                }
            }
        }
        de::Deserializer::deserialize_enum(
            deserializer,
            "DiffCommandValueTest",
            VARIANTS,
            Visitor {
                seed: self.val_wrapper,
                lifetime: std::marker::PhantomData,
            },
        )
    }
}

impl<'a, 'de, T: Deserialize<'de>> de::DeserializeSeed<'de> for DeserWrapper<'a, T> {
    type Value = Self;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        Deserialize::deserialize_in_place(deserializer, self.val)?;
        Ok(self)
    }
}

// Deserializes a DiffCommand but ignores values
pub(crate) struct DiffCommandIgnoreValue;

impl<'de> de::DeserializeSeed<'de> for DiffCommandIgnoreValue {
    type Value = DiffCommandValue<'de, ()>;
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor<'de> {
            lifetime: std::marker::PhantomData<&'de ()>,
        }
        impl<'de> de::Visitor<'de> for Visitor<'de> {
            type Value = DiffCommandValue<'de, ()>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::fmt::Formatter::write_str(formatter, "enum DiffCommandValueTest")
            }
            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: de::EnumAccess<'de>,
            {
                match de::EnumAccess::variant(data)? {
                    (DiffCommandField::Enter, variant) => {
                        let enter =
                            de::VariantAccess::newtype_variant::<DiffPathElementValue>(variant)?;
                        Ok(DiffCommandValue::Enter(enter))
                    }
                    (DiffCommandField::Value, variant)
                    | (DiffCommandField::AddKey, variant)
                    | (DiffCommandField::EnterKey, variant)
                    | (DiffCommandField::RemoveKey, variant) => {
                        de::VariantAccess::newtype_variant::<de::IgnoredAny>(variant)?;
                        Ok(DiffCommandValue::Value(()))
                    }
                    (DiffCommandField::Remove, variant) => {
                        let num_elements = de::VariantAccess::newtype_variant::<usize>(variant)?;
                        Ok(DiffCommandValue::Remove(num_elements))
                    }
                    (DiffCommandField::Exit, variant) => {
                        de::VariantAccess::unit_variant(variant)?;
                        Ok(DiffCommandValue::Exit)
                    }
                }
            }
        }
        de::Deserializer::deserialize_enum(
            deserializer,
            "DiffCommandValueTest",
            VARIANTS,
            Visitor {
                lifetime: std::marker::PhantomData,
            },
        )
    }
}

#[doc(hidden)]
#[derive(Serialize, Debug)]
pub enum DiffCommandRef<'a, T: Serialize> {
    /// Enter a path element
    Enter(DiffPathElementValue<'a>),
    /// A value to be deserialized.
    /// For collections, this implies "add to end" if not preceded by UpdateIndex.
    Value(&'a T),
    /// Remove N items from end of collection
    Remove(usize),
    /// Add a key to a map
    AddKey(&'a T),
    /// Enter a key in a map.
    /// This isn't part of Enter because then DiffPathElementValue would have to be generic over T
    // Fortunately, keys on HashMaps are terminal as far as we're concerned.
    EnterKey(&'a T),
    /// Remove a key from a map
    RemoveKey(&'a T),
    /// Exit a path element
    Exit,
}

#[doc(hidden)]
#[derive(Deserialize, Debug)]
pub enum DiffCommandValue<'a, T> {
    // Enter a path element
    #[serde(borrow)]
    Enter(DiffPathElementValue<'a>),
    /// A value to be deserialized.
    Value(T),
    /// Remove N items from end of collection
    Remove(usize),
    /// Add a key to a map
    AddKey(T),
    // Enter a key in a map
    EnterKey(T),
    /// Remove a key from a map
    RemoveKey(T),
    // Exit a path element
    Exit,
    // Never serialized
    Nothing,
    // Never serialized, used to indicate that deserializer wrote a value into supplied reference
    DeserializedValue,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Debug)]
pub enum DiffPathElementValue<'a> {
    /// A struct field
    #[serde(borrow)]
    Field(Cow<'a, str>),
    FieldIndex(u16),
    EnumVariant(Cow<'a, str>),
    FullEnumVariant,
    CollectionIndex(usize),
    AddToCollection,
}

impl<T: SerdeDiff + Serialize + for<'a> Deserialize<'a>> SerdeDiff for Vec<T> {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<bool, S::Error> {
        let mut self_iter = self.iter();
        let mut other_iter = other.iter();
        let mut idx = 0;
        let mut need_exit = false;
        let mut changed = false;
        loop {
            let self_item = self_iter.next();
            let other_item = other_iter.next();
            match (self_item, other_item) {
                (None, None) => break,
                (Some(_), None) => {
                    let mut num_to_remove = 1;
                    while self_iter.next().is_some() {
                        num_to_remove += 1;
                    }
                    ctx.save_command::<()>(&DiffCommandRef::Remove(num_to_remove), true, true)?;
                    changed = true;
                    need_exit = false;
                }
                (None, Some(other_item)) => {
                    ctx.save_command::<()>(
                        &DiffCommandRef::Enter(DiffPathElementValue::AddToCollection),
                        false,
                        true,
                    )?;
                    ctx.save_command(&DiffCommandRef::Value(other_item), true, true)?;
                    need_exit = true;
                    changed = true;
                }
                (Some(self_item), Some(other_item)) => {
                    ctx.push_collection_index(idx);
                    if <T as SerdeDiff>::diff(self_item, ctx, other_item)? {
                        need_exit = true;
                        changed = true;
                    }
                    ctx.pop_path_element()?;
                }
            }
            idx += 1;
        }
        if need_exit {
            ctx.save_command::<()>(&DiffCommandRef::Exit, true, false)?;
        }
        Ok(changed)
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<bool, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut changed = false;
        while let Some(cmd) = ctx.read_next_command::<A, T>(seq)? {
            use DiffCommandValue::*;
            use DiffPathElementValue::*;
            match cmd {
                // we should not be getting fields when reading collection commands
                Enter(Field(_)) => {
                    ctx.skip_value(seq)?;
                    break;
                }
                Enter(CollectionIndex(idx)) => {
                    if let Some(value_ref) = self.get_mut(idx) {
                        changed |= <T as SerdeDiff>::apply(value_ref, seq, ctx)?;
                    } else {
                        ctx.skip_value(seq)?;
                    }
                }
                Enter(AddToCollection) => {
                    if let Value(v) = ctx
                        .read_next_command(seq)?
                        .expect("Expected value after AddToCollection")
                    {
                        changed = true;
                        self.push(v);
                    } else {
                        panic!("Expected value after AddToCollection");
                    }
                }
                Remove(num_elements) => {
                    let new_length = self.len().saturating_sub(num_elements);
                    self.truncate(new_length);
                    changed = true;
                    break;
                }
                _ => break,
            }
        }
        Ok(changed)
    }
}
