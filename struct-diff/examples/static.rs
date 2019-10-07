use struct_diff_derive::Diffable;

//
// The basic trait for anything that's diffable
//
trait DiffableByCustom<T, DiffT> {
    fn diff(old: &T, new: &T) -> DiffT;
    fn apply(diff: &DiffT, target: &mut T);
}

trait DiffableByClone<T, DiffT> {
    fn diff(old: &T, new: &T) -> DiffT;
    fn apply(diff: &DiffT, target: &mut T);
}

trait DiffableByCopy<T, DiffT> {
    fn diff(old: &T, new: &T) -> DiffT;
    fn apply(diff: &DiffT, target: &mut T);
}

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

impl<T> DiffableByCopy<T, Option<T>> for T where T: PartialEq + Copy {
    fn diff(old: &T, new: &T) -> Option<T> {
        if old != new {
            Some(*new)
        } else {
            None
        }
    }

    fn apply(diff: &Option<T>, target: &mut T) {
        if let Some(value) = diff {
            *target = *value;
        }
    }
}


//
// Blanket impl for Clone, did not work
//

impl<T> DiffableByClone<T, Option<T>> for T where T: PartialEq + Clone {
    fn diff(old: &T, new: &T) -> Option<T> {
        if old != new {
            Some(new.clone())
        } else {
            None
        }
    }

    fn apply(diff: &Option<T>, target: &mut T) {
        if let Some(value) = diff {
            *target = value.clone();
        }
    }
}

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


//
// Example custom implementation for a Vec. This is a bit of a contrived example because it's unclear
// how to handle removing elements or adding new elements that are at an index greater than the length
// of the target Vec.
//
#[derive(Debug)]
struct FixedLengthVecDiff<T> {
    values: Vec<(usize, T)>,
    count: usize
}

impl<T> DiffableByCustom<Vec<T>, Option<FixedLengthVecDiff<T>>> for Vec<T> where T: PartialEq + Clone {
    fn diff(old: &Vec<T>, new: &Vec<T>) -> Option<FixedLengthVecDiff<T>> {
        let mut values = vec![];

        // Diffing a Vec in this way doesn't make much sense unless the vec lengths are the same.
        // If they aren't the same, using a proper key-value store is a better fit
        assert!(old.len() == new.len());

        for i in 0..new.len() {
            match old.get(i) {
                Some(old_value) => {
                    if old_value != &new[i] {
                        values.push((i, new[i].clone()));
                    }
                },
                None => {
                    values.push((i, new[i].clone()));
                }
            }
        }

        if values.len() > 0 || old.len() != new.len() {
            Some(FixedLengthVecDiff {
                values,
                count: new.len()
            })
        } else {
            None
        }
    }

    fn apply(diff: &Option<FixedLengthVecDiff<T>>, target: &mut Vec<T>) {
        if let Some(diff) = diff {
            assert!(diff.count == target.len());

            for v in &diff.values {
                target[v.0] = v.1.clone();
            }
        }
    }
}


#[derive(Diffable, Clone, PartialEq, Debug)]
struct MyInnerStruct {
    x: f32,

    #[diffable(clone)]
    a_string: String,

    #[diffable(custom, diff_type="FixedLengthVecDiff<String>")]
    string_list: Vec<String>
}

#[derive(Diffable, Debug)]
struct MyStruct {
    a: f32,
    b: i32,

    #[diffable(clone)]
    s: String,

    #[diffable(custom)]
    c: MyInnerStruct
}


fn main() {
    // Create old state
    let mut old = MyStruct {
        a: 5.0,
        b: 32,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str2".to_string()]
        }
    };

    // Create new state
    let mut new = MyStruct {
        a: 5.0,
        b: 32,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str2".to_string()]
        }
    };

    // Create a diff
    let diff = MyStruct::diff(&old, &new);
    assert!(diff.is_none());

    new.b = 33;

    let diff = MyStruct::diff(&old, &new);
    assert!(diff.is_some());

    println!("diff: {:?}", diff);
    println!("  before: {:?}", old);
    MyStruct::apply(&diff, &mut old);
    println!("  after : {:?}", old);

    assert!(old.b == 33);

    new.c.string_list = vec!["str1".to_string(), "str2_edited".to_string()];

    let diff = MyStruct::diff(&old, &new);
    println!("diff: {:?}", diff);

    println!("  before: {:?}", old);
    MyStruct::apply(&diff, &mut old);
    println!("  after : {:?}", old);
}





//
// This bit would be generated by a proc macro on MyStruct
//

/*
#[derive(Default)]
struct MyStructDiff {
    a: Option<f32>,
    b: Option<i32>,
    c: Option<MyInnerStructDiff>
}
*/
/*
impl DiffableByCustom<MyStruct, Option<MyStructDiff>> for MyStruct {
    fn diff(old: &MyStruct, new: &MyStruct) -> Option<MyStructDiff> {
        let mut struct_diff = MyStructDiff::default();
        let mut has_change = false;

        {
            let member_diff = <f32 as DiffableByCopy<_, _>>::diff(&old.a, &new.a);
            if member_diff.is_some() {
                struct_diff.a = member_diff;
                has_change = true;
            }
        }

        {
            let member_diff = <i32 as DiffableByCopy<_, _>>::diff(&old.b, &new.b);
            if member_diff.is_some() {
                struct_diff.b = member_diff;
                has_change = true;
            }
        }

        {
            let member_diff = <MyInnerStruct as DiffableByCustom<_, _>>::diff(&old.c, &new.c);
            if member_diff.is_some() {
                struct_diff.c = member_diff;
                has_change = true;
            }
        }

        if has_change {
            Some(struct_diff)
        } else {
            None
        }
    }

    fn apply(diff: &Option<MyStructDiff>, target: &mut MyStruct) {
        if let Some(diff) = diff {
            if let Some(a) = diff.a {
                target.a = a;
            }

            if let Some(b) = diff.b {
                target.b = b;
            }
        }
    }
}
*/







//
// This bit would be generated by a proc macro on MyInnerStruct
//

/*
#[derive(Default)]
struct MyInnerStructDiff {
    x: Option<f32>,
    a_string: Option<String>,
    string_list: Option<VecDiff<String>>
}
*/
/*
impl DiffableByCustom<MyInnerStruct, Option<MyInnerStructDiff>> for MyInnerStruct {
    fn diff(old: &MyInnerStruct, new: &MyInnerStruct) -> Option<MyInnerStructDiff> {
        let mut struct_diff = MyInnerStructDiff::default();
        let mut has_change = false;

        {
            let member_diff = <f32 as DiffableByCopy<_, _>>::diff(&old.x, &new.x);
            if member_diff.is_some() {
                struct_diff.x = member_diff;
                has_change = true;
            }
        }

        {
            let member_diff = <String as DiffableByClone<_, _>>::diff(&old.a_string, &new.a_string);
            if member_diff.is_some() {
                struct_diff.a_string = member_diff;
                has_change = true;
            }
        }

        {
            let member_diff = <Vec<String> as DiffableByCustom<_, _>>::diff(&old.string_list, &new.string_list);
            if member_diff.is_some() {
                struct_diff.string_list = member_diff;
                has_change = true;
            }
        }

        if has_change {
            Some(struct_diff)
        } else {
            None
        }
    }

    fn apply(diff: &Option<MyInnerStructDiff>, target: &mut MyInnerStruct) {
        if let Some(diff) = diff {
            if let Some(x) = diff.x {
                target.x = x;
            }
        }
    }
}
*/