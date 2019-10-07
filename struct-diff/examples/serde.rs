use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};

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
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error>;

    //TODO: This takes value by value because serde_json::from_value does too.
    fn apply_json(&mut self, path: &[DiffPathElementValue], depth: usize, value: serde_json::Value) -> Result<(), serde_json::Error>;
}

/// Used to describe a location within a struct. Typically this would be a member
/// name but could potentially be a tuple index, Vec index, HashMap key, etc.
#[derive(Serialize, Deserialize)]
enum DiffPathElementRef {
    /// A struct field
    Field(&'static str),
}

/// Describes a value of interest within a struct, and a reference to the data
#[derive(Serialize)]
struct DiffElementRef<'a, 's, T: Serialize> {
    /// Identifier for the location of the value
    path: &'s Vec<DiffPathElementRef>,

    /// Reference to the value within the struct
    element: &'a T,
}

/// Used during a diff operation for transient data used during the diff
struct DiffContext<'a, S: SerializeSeq> {
    /// A stack of identifiers that is maintained while we walk through the data recursively
    field_stack: Vec<DiffPathElementRef>,

    /// Reference to the serializer used to save the data
    serializer: &'a mut S,
}

impl<'a, S: SerializeSeq> DiffContext<'a, S> {
    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_field
    /// is called. See `pop_field`
    fn push_field(&mut self, field_name: &'static str) {
        self.field_stack.push(DiffPathElementRef::Field(field_name));
    }

    /// Called when we finish visiting a field. See `push_field` for details
    fn pop_field(&mut self) {
        self.field_stack.pop();
    }

    /// Adds a `DiffElement` to the context. `DiffElements` contain the path to the data and a
    /// reference to that data. This can be used later to store the changed values.
    fn save_value<T: Serialize>(&mut self, value: &T) -> Result<(), S::Error> {
        let element = DiffElementRef {
            path: &self.field_stack,
            element: value,
        };

        self.serializer.serialize_element(&element)
    }
}


#[derive(Deserialize, Debug)]
enum DiffPathElementValue {
    /// A struct field
    Field(String),
}

#[derive(Deserialize, Debug)]
struct DiffElementValueJson {
    path: Vec<DiffPathElementValue>,
    element: serde_json::Value,
}

//struct ApplyContext<'a, S: for<'de> Deserialize<'de>> {
//    field_stack: Vec<DiffElementValue>,
//    deserializer: &'a S
//}

#[derive(Clone, PartialEq, Debug)]
struct MyInnerStruct {
    //#[serde_diffable(skip)]
    x: f32,

    //#[serde_diffable(skip)]
    a_string: String,

    //#[serde_diffable(skip)]
    string_list: Vec<String>,
}

//#[derive(SerdeDiffable)]
#[derive(Debug)]
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
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(other)?;
        }

        Ok(())
    }

    fn apply_json(&mut self, _path: &[DiffPathElementValue], depth: usize, value: serde_json::Value) -> Result<(), serde_json::Error> {
        debug_assert!(_path.len() == depth);
        *self = serde_json::from_value(value)?;
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
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
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

    fn apply_json(&mut self, path: &[DiffPathElementValue], depth: usize, value: serde_json::Value) -> Result<(), serde_json::Error> {
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
}



impl SerdeDiffable for MyInnerStruct {
    fn diff<'a, S: SerializeSeq>(&self, _ctx: &mut DiffContext<'a, S>, _other: &Self) -> Result<(), S::Error> {
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

    fn apply_json(&mut self, path: &[DiffPathElementValue], depth: usize, _value: serde_json::Value) -> Result<(), serde_json::Error> {
        match &path[depth] {
            DiffPathElementValue::Field(_field_name) => {
            }
        }

        // Unmatched values will be silently ignored
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

    let data : Vec<DiffElementValueJson> = serde_json::from_str(&utf8_data).unwrap();
    println!("{:?}", data);
    println!("target: {:?}", target);

    for d in &data {
        target.apply_json(&d.path, 0, d.element.clone()).unwrap();
    }

    println!("target: {:?}", target);
}
