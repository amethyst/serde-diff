use serde::{ser::SerializeSeq, Deserialize, Serialize, Serializer};

use struct_diff_derive::SerdeDiffable;

/// Anything diffable implements this trait
trait SerdeDiffable {
    /// Recursively walk the struct, invoking serialize_element on each member if the element is
    /// different.
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error>;

    //fn apply(&mut self, ctx: &mut ApplyContext);
}

/// Used to describe a location within a struct. Typically this would be a member
/// name but could potentially be a tuple index, Vec index, HashMap key, etc.
#[derive(Serialize, Deserialize)]
enum DiffPathElement {
    /// A struct field
    Field(&'static str),
}

/// Describes a value of interest within a struct, and a reference to the data
#[derive(Serialize)]
struct DiffElement<'a, 's, T: Serialize> {
    /// Identifier for the location of the value
    path: &'s Vec<DiffPathElement>,

    /// Reference to the value within the struct
    element: &'a T,
}

/// Used during a diff operation for transient data used during the diff
struct DiffContext<'a, S: SerializeSeq> {
    /// A stack of identifiers that is maintained while we walk through the data recursively
    field_stack: Vec<DiffPathElement>,

    /// Reference to the serializer used to save the data
    serializer: &'a mut S,
}


impl<'a, S: SerializeSeq> DiffContext<'a, S> {
    /// Called when we visit a field. If the structure is recursive (i.e. struct within struct,
    /// elements within an array) this may be called more than once before a corresponding pop_field
    /// is called. See `pop_field`
    fn push_field(&mut self, field_name: &'static str) {
        self.field_stack.push(DiffPathElement::Field(field_name));
    }

    /// Called when we finish visiting a field. See `push_field` for details
    fn pop_field(&mut self) {
        self.field_stack.pop();
    }

    /// Adds a `DiffElement` to the context. `DiffElements` contain the path to the data and a
    /// reference to that data. This can be used later to store the changed values.
    fn save_value<T: Serialize>(&mut self, value: &T) -> Result<(), S::Error> {
        let element = DiffElement {
            path: &self.field_stack,
            element: value,
        };

        self.serializer.serialize_element(&element)
    }
}

//struct ApplyContext<'a, S: SerializeSeq> {
//    field_stack: Vec<DiffPathElement>,
//    serializer: &'a mut S,
//}

#[derive(Clone, PartialEq, Debug, SerdeDiffable)]
struct MyInnerStruct {
    x: f32,

    a_string: String,

    #[serde_diffable(skip)]
    string_list: Vec<String>,
}

#[derive(SerdeDiffable)]
struct MyStruct {
    a: f32,
    b: i32,
    s: String,
    c: MyInnerStruct,
}

impl SerdeDiffable for i32 {
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(self)?;
        }

        Ok(())
    }
}
impl SerdeDiffable for f32 {
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(self)?;
        }

        Ok(())
    }
}
impl SerdeDiffable for String {
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) -> Result<(), S::Error> {
        if self != other {
            ctx.save_value(&self)?;
        }

        Ok(())
    }
}
//impl SerdeDiffable for Vec<String> {
//    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) {
//        if self != other {
//            ctx.save_value(&self);
//        }
//    }
//}



//
// This is emitted by deriving SerdeDiffable
//
/*
impl SerdeDiffable for MyStruct {
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) {
        ctx.push_field("a");
        self.a.diff(ctx, &other.a);
        ctx.pop();
        ctx.push_field("b");
        self.b.diff(ctx, &other.b);
        ctx.pop();
        ctx.push_field("s");
        self.s.diff(ctx, &other.s);
        ctx.pop();
        ctx.push_field("c");
        self.c.diff(ctx, &other.c);
        ctx.pop();
    }
}
*/

/*
impl SerdeDiffable for MyInnerStruct {
    fn diff<'a, S: SerializeSeq>(&self, ctx: &mut DiffContext<'a, S>, other: &Self) {
        ctx.push_field("x");
        self.x.diff(ctx, &other.x);
        ctx.pop();
        ctx.push_field("a_string");
        self.a_string.diff(ctx, &other.a_string);
        ctx.pop();
        // ctx.push_field("string_list");
        // self.string_list.diff(ctx, &other.string_list);
        // ctx.pop();
    }
}
*/

//
// These impls would have been nice but I found them problematic:
// - We don't want to blanket impl for every Clone. Sometimes it's nice (like String) but sometimes
//   it's undesirable. Vec would be better implemented with custom logic.
// - It could be reasonable to blanket impl for every Copy. Unfortunately, if we do this and then
//   try to custom impl for anything else (for example, String) the compiler will fail because it's
//   concerned that String could become Copy in the future ("upstream crates may add new impl of trait
//   `std::marker::Copy` for type `std::string::String` in future versions")
// - Additionally, trying to blanket impl for both Copy and Clone fails because for Copy values,
//   the impl that should be used is ambiguous
// - Also tried a separate "AllowCopyDiff" and "AllowCloneDiff" that impl Diffable. This works, but
//   even if I impl AllowCopyDiff or AllowCloneDiff, I can't do something like
//   <f32 as Diffable>::diff(...) which makes implementing the macro for making a struct difficult
//   complex, and would require extra markup. So this approach has no benefit over using macros to
//   impl directly for the type (i.e. f32).
// - If we get specialization in Rust, there would likely be new approaches that could work that avoid
//   macros.

//
// Attempted to use a custom type to avoid macros, did not work
//
/*
trait AllowCopyDiff<T : PartialEq + Copy> {}
trait AllowCloneDiff<T : PartialEq + Clone> {}

impl AllowCopyDiff<f32> for f32 {}
impl AllowCopyDiff<i32> for i32 {}
impl AllowCloneDiff<String> for String {}

impl<U> Diffable<U, Option<U>> for dyn AllowCopyDiff<U>
    where
        U: PartialEq + Copy
{
    fn diff(old: &U, new: &U) -> Option<U> {
        if old != new {
            Some(*new)
        } else {
            None
        }
    }

    fn apply(diff: &Option<U>, target: &mut U) {
        if let Some(value) = diff {
            *target = value.clone();
        }
    }
}

impl<U> Diffable<U, Option<U>> for dyn AllowCloneDiff<U>
    where
        U: PartialEq + Clone
{
    fn diff(old: &U, new: &U) -> Option<U> {
        if old != new {
            Some(new.clone())
        } else {
            None
        }
    }

    fn apply(diff: &Option<U>, target: &mut U) {
        if let Some(value) = diff {
            *target = value.clone();
        }
    }
}
*/

//
// Blanket impl for Copy, did not work
//
/*
macro_rules! allow_copy_diff {
    ($t:ty) => {
        impl DiffableByCopy<$t, Option<$t>> for $t {
            fn diff(old: &$t, new: &$t) -> Option<$t> {
                if old != new {
                    Some(*new)
                } else {
                    None
                }
            }

            fn apply(diff: &Option<$t>, target: &mut $t) {
                if let Some(value) = diff {
                    *target = *value;
                }
            }
        }
    }
}

macro_rules! allow_clone_diff {
    ($t:ty) => {
        impl DiffableByClone<$t, Option<$t>> for $t {
            fn diff(old: &$t, new: &$t) -> Option<$t> {
                if old != new {
                    Some(new.clone())
                } else {
                    None
                }
            }

            fn apply(diff: &Option<$t>, target: &mut $t) {
                if let Some(value) = diff {
                    *target = value.clone();
                }
            }
        }
    }
}

allow_copy_diff!(f32);
allow_copy_diff!(f64);

allow_copy_diff!(i8);
allow_copy_diff!(i16);
allow_copy_diff!(i32);
allow_copy_diff!(i64);
allow_copy_diff!(i128);

allow_copy_diff!(u8);
allow_copy_diff!(u16);
allow_copy_diff!(u32);
allow_copy_diff!(u64);
allow_copy_diff!(u128);

allow_clone_diff!(String);
allow_clone_diff!(std::path::PathBuf);
*/

fn main() {
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
    println!("{}", String::from_utf8(vec).unwrap());

    //     // Create a diff
    //     let diff = MyStruct::diff(&old, &new);
    //     assert!(diff.is_none());

    //     new.b = 33;

    //     let diff = MyStruct::diff(&old, &new);
    //     assert!(diff.is_some());

    //     println!("{:?}", diff);
    //     MyStruct::apply(&diff, &mut old);

    //     assert!(old.b == 33);

    //     new.c.string_list = vec!["str1".to_string(), "str2_edited".to_string()];

    //     let diff = MyStruct::diff(&old, &new);
    //     println!("{:?}", diff);
}
