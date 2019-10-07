use serde::{de, ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use serde_derive::*;
use std::borrow::Cow;

//use struct_diff_derive::SerdeDiffable;

// NEXT STEPS:
// - Decouple from serde_json as much as possible. We might need to use a "stream" format with
//   well-defined data order to be able to use serde Deserializer trait.
// - Make all fields work again
// - Make it work via proc macro
// - Blanket impl or impl-via-macro common std types (i.e f32, i32, String)
// - Handle containers

//TODO: Currently we store data as a flat list, i.e. {Vec<Path>, Value}, {Vec<Path>, Value}
// This leads to redundant data and requires more lookups when we try to apply a diff
// It might be better to try to store it hierarchically:
// {
//     path: field1,
//     children: [
//         {
//             path: sub_field1,
//             value: "a value"
//         },
//         {
//             path: sub_field2,
//             value: "a value"
//         }
//     ]
// }
// Repeatedly following the full path could be painful if we had many fields, which could happen
// the data has a Vec deeply nested within the struct

/// Anything diffable implements this trait
trait SerdeDiffable {
    /// Recursively walk the struct, invoking serialize_element on each member if the element is
    /// different.
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error>;

    //TODO: This takes value by value because serde_json::from_value does too.
    fn apply_json(
        &mut self,
        path: &[DiffPathElementValue],
        depth: usize,
        value: serde_json::Value,
    ) -> Result<(), serde_json::Error>;

    fn apply<'de, A>(
        &mut self,
        seq: &mut A,
        ctx: &mut ApplyContext,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>;
}

/// Used to describe a location within a struct. Typically this would be a member
/// name but could potentially be a tuple index, Vec index, HashMap key, etc.
// #[derive(Serialize)]
// enum DiffPathElementRef {
//     /// A struct field
//     Field(&'static str),
// }

/// Describes a value of interest within a struct, and a reference to the data
#[derive(Serialize)]
struct DiffElementRef<'a, 's, T: Serialize> {
    /// Identifier for the location of the value
    path: &'s Vec<DiffPathElementValue<'a>>,

    /// Reference to the value within the struct
    element: &'a T,
}

/// Used during a diff operation for transient data used during the diff
struct DiffContext<'a, S: SerializeSeq> {
    /// A stack of identifiers that is maintained while we walk through the data recursively
    field_stack: Vec<DiffPathElementValue<'static>>,

    /// Reference to the serializer used to save the data
    serializer: &'a mut S,
}

impl<'a, S: SerializeSeq> DiffContext<'a, S> {
    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_field
    /// is called. See `pop_field`
    fn push_field(&mut self, field_name: &'static str) {
        let cmd =
            DiffCommandRef::<()>::Enter(DiffPathElementValue::Field(Cow::Borrowed(field_name)));
        self.serializer.serialize_element(&cmd);
    }

    /// Called when we finish visiting a field. See `push_field` for details
    fn pop_field(&mut self) {
        let cmd = DiffCommandRef::<()>::Exit;
        self.serializer.serialize_element(&cmd);
    }

    /// Adds a `DiffElement` to the context. `DiffElements` contain the path to the data and a
    /// reference to that data. This can be used later to store the changed values.
    fn save_value<T: Serialize>(&mut self, value: &T) -> Result<(), S::Error> {
        let element = DiffElementRef {
            path: &self.field_stack,
            element: value,
        };
        let cmd = DiffCommandRef::Value(value);
        self.serializer.serialize_element(&cmd)
    }
}

struct Diff<'a, 'b, T: SerdeDiffable + Serialize> {
    old: &'a T,
    new: &'b T,
}
impl<'a, 'b, T: SerdeDiffable + Serialize> Serialize for Diff<'a, 'b, T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        let mut ctx = DiffContext {
            serializer: &mut seq,
            field_stack: Vec::new(),
        };
        self.old.diff(&mut ctx, &self.new)?;
        Ok(seq.end()?)
    }
}
struct Apply<'a, T: SerdeDiffable + for<'c> Deserialize<'c>> {
    target: &'a mut T,
}
impl<'a, 'de, T: SerdeDiffable + for<'c> Deserialize<'c>> Deserialize<'de> for Apply<'a, T> {
    fn deserialize<D>(deserializer: D) -> Result<Apply<'a, T>, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        panic!("deserialize not supported - use deserialize_in_place!");
    }
    fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Apply {
            target: place.target,
        })?;
        Ok(())
    }
}
impl<'a, 'de, T: SerdeDiffable + for<'c> Deserialize<'c>> de::Visitor<'de> for Apply<'a, T> {
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
struct ApplyContext {}
impl ApplyContext {
    fn next_path_element<'de, A>(
        &mut self,
        seq: &mut A,
    ) -> Result<Option<DiffPathElementValue>, <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        use DiffCommandValue::*;
        match seq.next_element::<DiffCommandValue<()>>()? {
            Some(Enter(element)) => Ok(Some(element)),
            Some(Value(_)) => panic!("unexpected DiffCommand::Value"),
            Some(Exit) | None => Ok(None),
        }
    }
    fn skip_value<'de, A>(&mut self, seq: &mut A) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // TODO somehow skip the value without knowing the type - not possible for some formats, so should probably panic
        // also need to handle any enter/exits here
        unimplemented!()
    }
    fn read_value<'de, A, T: for<'c> Deserialize<'c>>(
        &mut self,
        seq: &mut A,
        val: &mut T,
    ) -> Result<(), <A as de::SeqAccess<'de>>::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if let Some(new_val) = seq.next_element::<DiffCommandValue<T>>()? {
            if let DiffCommandValue::Value(new_val) = new_val {
                *val = new_val;
            }
        }
        Ok(())
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
    Enter(DiffPathElementValue<'a>),
    Value(T),
    // Exit a path element
    Exit,
}
#[derive(Serialize, Deserialize, Debug)]
enum DiffPathElementValue<'a> {
    /// A struct field
    Field(Cow<'a, str>),
}

#[derive(Deserialize, Debug)]
struct DiffElementValueJson<'a> {
    path: Vec<DiffPathElementValue<'a>>,
    element: serde_json::Value,
}

//struct ApplyContext<'a, S: for<'de> Deserialize<'de>> {
//    field_stack: Vec<DiffElementValue>,
//    deserializer: &'a S
//}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
struct MyInnerStruct {
    //#[serde_diffable(skip)]
    x: f32,

    //#[serde_diffable(skip)]
    a_string: String,

    //#[serde_diffable(skip)]
    string_list: Vec<String>,
}

//#[derive(SerdeDiffable)]
#[derive(Serialize, Deserialize, Debug)]
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

    fn apply_json(
        &mut self,
        _path: &[DiffPathElementValue],
        depth: usize,
        value: serde_json::Value,
    ) -> Result<(), serde_json::Error> {
        debug_assert!(_path.len() == depth);
        *self = serde_json::from_value(value)?;
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

//impl SerdeDiffable for f32 {
//    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
//        if self != other {
//            ctx.save_value(other)?;
//        }
//
//        Ok(())
//    }
//}
//
//impl SerdeDiffable for String {
//    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
//        if self != other {
//            ctx.save_value(&other)?;
//        }
//
//        Ok(())
//    }
//}

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

impl SerdeDiffable for MyStruct {
    fn diff<'a, S: SerializeSeq>(
        &self,
        ctx: &mut DiffContext<'a, S>,
        other: &Self,
    ) -> Result<(), S::Error> {
        //        ctx.push_field("a");
        //        self.a.diff(ctx, &other.a)?;
        //        ctx.pop_field();
        ctx.push_field("b");
        self.b.diff(ctx, &other.b)?;
        ctx.pop_field();
        //        ctx.push_field("s");
        //        self.s.diff(ctx, &other.s)?;
        //        ctx.pop_field();
        //        ctx.push_field("c");
        //        self.c.diff(ctx, &other.c)?;
        //        ctx.pop_field();
        Ok(())
    }
    //    fn apply<S: for<'de> Deserialize<'de>>(&mut self, ctx: &ApplyContext<S>) {
    //
    //    }

    fn apply_json(
        &mut self,
        path: &[DiffPathElementValue],
        depth: usize,
        value: serde_json::Value,
    ) -> Result<(), serde_json::Error> {
        match &path[depth] {
            DiffPathElementValue::Field(field_name) => {
                if field_name == "b" {
                    self.b.apply_json(path, depth + 1, value)?;
                }
            }
        }

        // Unmatched values will be silently ignored
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
        while let Some(DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
            match element.as_ref() {
                // "a" => SerdeDiffable::apply(self.a, seq, ctx)?,
                "b" => self.b.apply(seq, ctx)?,
                "c" => self.c.apply(seq, ctx)?,
                _ => ctx.skip_value(seq)?,
            }
        }
        Ok(())
    }
}

impl SerdeDiffable for MyInnerStruct {
    fn diff<'a, S: SerializeSeq>(
        &self,
        _ctx: &mut DiffContext<'a, S>,
        _other: &Self,
    ) -> Result<(), S::Error> {
        //        ctx.push_field("x");
        //        self.x.diff(ctx, &other.x)?;
        //        ctx.pop_field();
        //        ctx.push_field("a_string");
        //        self.a_string.diff(ctx, &other.a_string)?;
        //        ctx.pop_field();
        // ctx.push_field("string_list");
        // self.string_list.diff(ctx, &other.string_list);
        // ctx.pop_field();
        Ok(())
    }
    //    fn apply<S: for<'de> Deserialize<'de>>(&mut self, ctx: &ApplyContext<S>) {
    //
    //    }

    fn apply_json(
        &mut self,
        path: &[DiffPathElementValue],
        depth: usize,
        _value: serde_json::Value,
    ) -> Result<(), serde_json::Error> {
        match &path[depth] {
            DiffPathElementValue::Field(_field_name) => {}
        }

        // Unmatched values will be silently ignored
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
        while let Some(DiffPathElementValue::Field(element)) = ctx.next_path_element(seq)? {
            match element.as_ref() {
                // "x" => SerdeDiffable::apply(self.x, seq, ctx)?,
                _ => ctx.skip_value(seq)?,
            }
        }
        Ok(())
    }
}

fn main() {
    let utf8_data = {
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
        // let mut seq = serializer.serialize_seq(None).unwrap();
        // let mut ctx = DiffContext {
        //     serializer: &mut seq,
        //     field_stack: Vec::new(),
        // };
        // old.diff(&mut ctx, &new).unwrap();
        // seq.end().unwrap();

        // Create new state
        let new = MyStruct {
            a: 5.0,
            b: 32,
            s: "A string".to_string(),
            c: MyInnerStruct {
                x: 39.0,
                a_string: "my string".to_string(),
                string_list: vec!["str1".to_string(), "str2".to_string()],
            },
        };
        let mut vec = Vec::new();
        let mut c = std::io::Cursor::new(&mut vec);
        let mut serializer = serde_json::Serializer::new(&mut c);
        let mut seq = serializer.serialize_seq(None).unwrap();
        let mut ctx = DiffContext {
            serializer: &mut seq,
            field_stack: Vec::new(),
        };
        old.diff(&mut ctx, &new).unwrap();
        seq.end().unwrap();

        let utf8_data = String::from_utf8(vec).unwrap();

        println!("{}", utf8_data);

        utf8_data
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

    let mut apply = Apply {
        target: &mut target,
    };
    let mut c = std::io::Cursor::new(utf8_data.as_bytes());
    let mut deserializer = serde_json::Deserializer::from_reader(c);
    let seq = deserializer.deserialize_seq(apply).unwrap();
    dbg!(&target);
    // <Apply<MyStruct> as Deserialize>::deserialize(deserializer);
    // let data: Vec<DiffElementValueJson> = serde_json::from_str(&utf8_data).unwrap();
    // println!("{:?}", data);
    // println!("target: {:?}", target);

    // for d in &data {
    //     target.apply_json(&d.path, 0, d.element.clone()).unwrap();
    // }

    println!("target: {:?}", target);
}
