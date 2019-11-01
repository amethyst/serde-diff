use serde::{
    de,
    ser::{self, SerializeSeq},
    Deserialize, Serialize, Serializer,
};
use serde_derive::*;
use std::borrow::Cow;
use struct_diff_derive::*;

//use struct_diff_derive::SerdeDiffable;

// NEXT STEPS:
// - Decouple from serde_json as much as possible. We might need to use a "stream" format with
//   well-defined data order to be able to use serde Deserializer trait. DONE?
// - Make all fields work again
// - Make it work via proc macro
// - Blanket impl or impl-via-macro common std types (i.e f32, i32, String)
// - Handle containers
// - Ignore type mismatches instead of propagating the error

//TODO: Currently we store data as a command list that encodes the hierarchy, i.e.
// [{"Enter":{"Field":"a"}},{"Value":3.0},{"Enter":{"Field":"c"}},{"Enter":{"Field":"x"}},{"Value":39.0}]
// Value is decoded as an implicit Exit to avoid excessive Exits in the data stream.
// It could probably be smaller and more readable in a text-based format.
//
// A problem occurs when encoding the command stream for bincode:
// We need to know the size of the list before we start serializing.
// To do so, we probably want to implement the serde::ser::Serializer trait and
// make the implementation only count up every time an element is serialized, doing nothing else.

/// Anything diffable implements this trait
pub trait SerdeDiffable {
    /// Recursively walk the struct, invoking serialize_element on each member if the element is
    /// different.
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error>;

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>;
}

/// Used during a diff operation for transient data used during the diff
pub struct DiffContext<'a, S: SerializeSeq> {
    field_stack: Vec<DiffPathElementValue<'a>>,
    /// Reference to the serializer used to save the data
    serializer: &'a mut S,
    /// save_value is an implicit Exit, so we set a flag to avoid writing the next Exit
    field_written: bool,
}

impl<'a, S: SerializeSeq> DiffContext<'a, S> {
    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_path_element
    /// is called. See `pop_path_element`
    fn push_field(&mut self, field_name: &'static str) {
        self.field_stack
            .push(DiffPathElementValue::Field(Cow::Borrowed(field_name)));
    }

    fn push_index(&mut self, idx: usize) {
        self.field_stack
            .push(DiffPathElementValue::Index(idx));
    }

    /// Called when we finish visiting a field. See `push_field` for details
    fn pop_path_element(&mut self) -> Result<(), S::Error> {
        if self.field_stack.is_empty() {
            // if we don't have any buffered fields, we just write Exit command directly to the serializer
            // if we've just written a field, skip the Exit
            if !self.field_written {
                let cmd = DiffCommandRef::<()>::Exit;
                self.serializer.serialize_element(&cmd)
            } else {
                self.field_written = false;
                Ok(())
            }
        } else {
            self.field_stack.pop();
            Ok(())
        }
    }

    /// Adds a `DiffElement` to the context. `DiffElements` contain the path to the data and a
    /// reference to that data. This can be used later to store the changed values.
    fn save_value<T: Serialize>(&mut self, value: &T) -> Result<(), S::Error> {
        if !self.field_stack.is_empty() {
            // flush buffered fields as Enter commands
            for field in self.field_stack.drain(0..self.field_stack.len()) {
                self.serializer
                    .serialize_element(&DiffCommandRef::<()>::Enter(field))?;
            }
        }
        self.field_written = true;
        let cmd = DiffCommandRef::Value(value);
        self.serializer.serialize_element(&cmd)
    }
}

/// Serializes the difference between two values of a type
pub struct Diff<'a, 'b, T> {
    old: &'a T,
    new: &'b T,
}
impl<'a, 'b, T: SerdeDiffable + 'a + 'b> Diff<'a, 'b, T> {
    fn serializable(old: &'a T, new: &'b T) -> Self {
        Self { old, new }
    }
    fn diff<S: Serializer>(serializer: S, old: &'a T, new: &'b T) -> Result<S::Ok, S::Error> {
        Self::serializable(old, new).serialize(serializer)
    }
}
impl<'a, 'b, T: SerdeDiffable> Serialize for Diff<'a, 'b, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let num_elements = {
            let mut serializer = CountingSerializer { num_elements: 0 };
            let mut seq = serializer.serialize_seq(None).unwrap();
            let mut ctx = DiffContext {
                field_stack: Vec::new(),
                serializer: &mut seq,
                field_written: false,
            };
            self.old.diff(&mut ctx, &self.new).unwrap();
            seq.end().unwrap();
            serializer.num_elements
        };
        let mut seq = serializer.serialize_seq(Some(num_elements))?;
        let mut ctx = DiffContext {
            field_stack: Vec::new(),
            serializer: &mut seq,
            field_written: false,
        };
        self.old.diff(&mut ctx, &self.new)?;
        Ok(seq.end()?)
    }
}

/// Deserializes a [Diff]
pub struct Apply<'a, T: SerdeDiffable> {
    target: &'a mut T,
}
impl<'a, 'de, T: SerdeDiffable> Apply<'a, T> {
    fn deserializable(target: &'a mut T) -> Self {
        Self { target }
    }
    fn apply<D>(deserializer: D, target: &mut T) -> Result<(), <D as de::Deserializer<'de>>::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Apply { target })
    }
}
impl<'a, 'de, T: SerdeDiffable> de::DeserializeSeed<'de> for Apply<'a, T> {
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

impl<'a, 'de, T: SerdeDiffable> de::Visitor<'de> for Apply<'a, T> {
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
            Some(Value(_)) => panic!("unexpected DiffCommand::Value"),
            Some(Exit) | Some(Nothing) | None => Ok(None),
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
        // TODO somehow skip the value without knowing the type - not possible for some formats, so should probably panic
        while let Some(cmd) = seq.next_element_seed(DiffCommandIgnoreValue {})? {
            match cmd {
                DiffCommandValue::Enter(_) => depth += 1,
                DiffCommandValue::Exit => depth -= 1,
                DiffCommandValue::Value(_) => depth -= 1, // ignore value, but reduce depth, as it is an implicit Exit
                DiffCommandValue::Nothing => panic!("should never serialize Nothing"),
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
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // The visitor handles enum cases and returns a command if it was not a Value
        let cmd = seq.next_element_seed::<DiffCommandDeserWrapper<T>>(DiffCommandDeserWrapper {
            val_wrapper: DeserWrapper { val },
        })?;
        match cmd {
            Some(DiffCommandValue::Enter(_)) => {
                self.skip_value_internal(seq, 1)?;
            }
            Some(DiffCommandValue::Exit) => panic!("unexpected Exit command"),
            _ => {}
        }

        Ok(())
    }
}

struct DeserWrapper<'a, T> {
    val: &'a mut T,
}
struct DiffCommandDeserWrapper<'a, T> {
    val_wrapper: DeserWrapper<'a, T>,
}

// This monstrosity is based off the output of the derive macro for DiffCommand.
// The justification for this is that we want to use Deserialize::deserialize_in_place
// for DiffCommand::Value in order to support zero-copy deserialization of T.
// This is achieved by passing &mut T through the DiffCommandDeserWrapper, which parsers the enum
// to the DeserWrapper which calls Deserialize::deserialize_in_place.
#[allow(non_camel_case_types)]
enum DiffCommandField {
    Enter,
    Value,
    Exit,
}
struct DiffCommandFieldVisitor;
const VARIANTS: &'static [&'static str] = &["Enter", "Value", "Exit"];
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
            2u64 => Ok(DiffCommandField::Exit),
            _ => Err(de::Error::invalid_value(
                de::Unexpected::Unsigned(value),
                &"variant index 0 <= i < 3",
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
            b"Exit" => Ok(DiffCommandField::Exit),
            _ => {
                let value = &serde::export::from_utf8_lossy(value);
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
                    (DiffCommandField::Value, variant) => {
                        de::VariantAccess::newtype_variant_seed::<DeserWrapper<T>>(
                            variant, self.seed,
                        )?;
                        Ok(DiffCommandValue::Nothing)
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
struct DiffCommandIgnoreValue;
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
                    (DiffCommandField::Value, variant) => {
                        de::VariantAccess::newtype_variant::<de::IgnoredAny>(variant)?;
                        Ok(DiffCommandValue::Value(()))
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

#[derive(Serialize, Debug)]
enum DiffCommandRef<'a, T: Serialize> {
    // Enter a path element
    Enter(DiffPathElementValue<'a>),
    Value(&'a T),
    // Exit a path element
    Exit,
}
#[derive(Deserialize, Debug)]
enum DiffCommandValue<'a, T> {
    // Enter a path element
    #[serde(borrow)]
    Enter(DiffPathElementValue<'a>),
    Value(T),
    // Exit a path element
    Exit,
    // Never serialized
    Nothing,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum DiffPathElementValue<'a> {
    /// A struct field
    #[serde(borrow)]
    Field(Cow<'a, str>),
    /// A collection index
    Index(usize),
}

#[derive(SerdeDiffable, Clone, PartialEq, Serialize, Deserialize, Debug)]
struct MyInnerStruct {
    //#[serde_diffable(skip)]
    x: f32,

    //#[serde_diffable(skip)]
    a_string: String,

    //#[serde_diffable(skip)]
    string_list: Vec<String>,
}

#[derive(SerdeDiffable, Serialize, Deserialize, Debug)]
struct MyStruct {
    //#[serde_diffable(skip)]
    a: f32,
    b: i32,
    //#[serde_diffable(skip)]
    s: String,
    //#[serde_diffable(skip)]
    c: MyInnerStruct,
}

impl SerdeDiffable for i32 {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(other)?;
        }

        Ok(())
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        ctx.read_value(seq, self)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
pub enum CollectionDiffValue<T: Serialize + Deserialize> {
    Removed,
    Value(T)
}
#[derive(Serialize, Deserialize)]
pub enum CollectionDiffRef<'a, T: Serialize + Deserialize> {
    Removed,
    Value(&'a T)
}
impl<T: PartialEq + SerdeDiffable> SerdeDiffable for Vec<T>
{
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error> {
        let mut self_iter = <Self as IntoIterator>::into_iter(self);
        let mut other_iter = <Self as IntoIterator::into_iter(other);
        let mut idx = 0;
        loop {
            let self_item = self_iter.next();
            let other_item = other_iter.next();
            if self_item.is_none() && other_item.is_none() {
                break;
            }
            if self_item != other_item {
                let to_serialize = if let Some(item) = other_item {
                    CollectionDiffRef::Value(item);
                } else {
                    CollectionDiffRef::Removed
                };
                ctx.push_index(idx);
                <T as SerdeDiffable>::diff(ctx)
                ctx.pop_path_element();
            }
            idx += 1;
        }
        Ok(())
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        for item in self.into_iter() {
            
        }
        Ok(())
    }
}

impl SerdeDiffable for f32 {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(other)?;
        }

        Ok(())
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        ctx.read_value(seq, self)?;
        Ok(())
    }
}

impl SerdeDiffable for String {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(other)?;
        }

        Ok(())
    }

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        ctx.read_value(seq, self)?;
        Ok(())
    }
}

//impl SerdeDiffable for Vec<String> {
//    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) {
//        if self != other {
//            ctx.save_value(&other);
//        }
//    }
//}

//
// This is emitted by deriving SerdeDiffable
//
// impl SerdeDiffable for MyStruct {
//     fn diff<'a, S: SerializeSeq>(
//         &self,
//         ctx: &mut DiffContext<'a, S>,
//         other: &Self,
//     ) -> Result<(), S::Error> {
//         ctx.push_field("a");
//         self.a.diff(ctx, &other.a)?;
//         ctx.pop_path_element()?;
//         ctx.push_field("b");
//         self.b.diff(ctx, &other.b)?;
//         ctx.pop_path_element()?;
//         //        ctx.push_field("s");
//         //        self.s.diff(ctx, &other.s)?;
//         //        ctx.pop_path_element();
//         ctx.push_field("c");
//         self.c.diff(ctx, &other.c)?;
//         ctx.pop_path_element()?;
//         Ok(())
//     }

//     fn apply<'de, A>(
//         &mut self,
//         seq: &mut A,
//         ctx: &mut ApplyContext,
//     ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
//     where
//         A: de::SeqAccess<'de>,
//     {
//         while let Some(DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
//             match element.as_ref() {
//                 "a" => self.a.apply(seq, ctx)?,
//                 "b" => self.b.apply(seq, ctx)?,
//                 "c" => self.c.apply(seq, ctx)?,
//                 _ => ctx.skip_value(seq)?,
//             }
//         }
//         Ok(())
//     }
// }

// impl SerdeDiffable for MyInnerStruct {
//     fn diff<'a, S: SerializeSeq>(
//         &self,
//         ctx: &mut DiffContext<'a, S>,
//         other: &Self,
//     ) -> Result<(), S::Error> {
//         ctx.push_field("x");
//         self.x.diff(ctx, &other.x)?;
//         ctx.pop_path_element()?;
//         ctx.push_field("a_string");
//         self.a_string.diff(ctx, &other.a_string)?;
//         ctx.pop_path_element()?;
//         // ctx.push_field("string_list");
//         // self.string_list.diff(ctx, &other.string_list);
//         // ctx.pop_path_element();
//         Ok(())
//     }
//     //    fn apply<S: for<'de> Deserialize<'de>>(&mut self, ctx: &ApplyContext<S>) {
//     //
//     //    }

//     fn apply<'de, A>(
//         &mut self,
//         seq: &mut A,
//         ctx: &mut ApplyContext,
//     ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
//     where
//         A: de::SeqAccess<'de>,
//     {
//         while let Some(DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
//             match element.as_ref() {
//                 "x" => self.x.apply(seq, ctx)?,
//                 "a_string" => self.a_string.apply(seq, ctx)?,
//                 _ => ctx.skip_value(seq)?,
//             }
//         }
//         Ok(())
//     }
// }

fn main() {
    let (utf8_data, bincode_data) = {
        // Create old state
        let old = MyStruct {
            a: 5.0,
            b: 31,
            s: "A string".to_string(),
            c: MyInnerStruct {
                x: 40.0,
                a_string: "my string".to_string(),
                string_list: vec!["str1".to_string(), "str2".to_string()],
            },
        };

        // Create new state
        let new = MyStruct {
            a: 3.0,
            b: 32,
            s: "A string".to_string(),
            c: MyInnerStruct {
                x: 39.0,
                a_string: "my other string".to_string(),
                string_list: vec!["str1".to_string(), "str2".to_string()],
            },
        };
        let mut vec = Vec::new();
        let mut c = std::io::Cursor::new(&mut vec);
        let mut serializer = serde_json::Serializer::new(&mut c);
        Diff::diff(&mut serializer, &old, &new).unwrap();

        let bincode_data = bincode::serialize(&Diff::serializable(&old, &new)).unwrap();
        // let bincode_data = vec![0];

        let utf8_data = String::from_utf8(vec).unwrap();

        println!("{}", utf8_data);
        println!("old: {:?} new {:?}", old, new);

        (utf8_data, bincode_data)
    };

    let mut target = MyStruct {
        a: 5.0,
        b: 31,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str2".to_string()],
        },
    };
    // let c = std::io::Cursor::new(utf8_data.as_bytes());
    // let mut deserializer = serde_json::Deserializer::from_reader(c);
    // Apply::apply(&mut deserializer, &mut target).unwrap();

    bincode::config()
        .deserialize_seed(Apply::deserializable(&mut target), &bincode_data)
        .unwrap();
    println!("target {:?}", target);
    println!(
        "bincode size {} json size {}",
        bincode_data.len(),
        utf8_data.len()
    );
    println!("str {:?}", std::str::from_utf8(&bincode_data).unwrap());
}

struct CountingSerializer {
    num_elements: usize,
}

#[derive(Debug)]
struct SerializerError;
impl std::fmt::Display for SerializerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}
impl std::error::Error for SerializerError {
    fn description(&self) -> &str {
        ""
    }
    fn cause(&self) -> Option<&dyn std::error::Error> {
        None
    }
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
impl ser::Error for SerializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        SerializerError
    }
}

impl<'a> ser::Serializer for &'a mut CountingSerializer {
    type Ok = ();
    type Error = SerializerError;

    type SerializeSeq = Self;
    type SerializeTuple = ser::Impossible<(), Self::Error>;
    type SerializeTupleStruct = ser::Impossible<(), Self::Error>;
    type SerializeTupleVariant = ser::Impossible<(), Self::Error>;
    type SerializeMap = ser::Impossible<(), Self::Error>;
    type SerializeStruct = ser::Impossible<(), Self::Error>;
    type SerializeStructVariant = ser::Impossible<(), Self::Error>;

    fn serialize_bool(self, v: bool) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i8(self, v: i8) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i16(self, v: i16) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i32(self, v: i32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_i64(self, v: i64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u8(self, v: u8) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u16(self, v: u16) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u32(self, v: u32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_u64(self, v: u64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_f32(self, v: f32) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_f64(self, v: f64) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_char(self, v: char) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_str(self, v: &str) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_some<T>(self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_unit(self) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<(), Self::Error> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        unimplemented!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        unimplemented!()
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        unimplemented!()
    }
}

impl<'a> ser::SerializeSeq for &'a mut CountingSerializer {
    type Ok = ();
    type Error = SerializerError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        self.num_elements += 1;
        Ok(())
    }

    fn end(self) -> Result<(), Self::Error> {
        Ok(())
    }
}
