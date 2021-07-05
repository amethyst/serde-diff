use crate as serde_diff;
use crate::{Apply, Diff, SerdeDiff};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::Debug;

#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug, Copy, Clone)]
struct TestStruct {
    a: u32,
    b: f64,
}

fn roundtrip<T: SerdeDiff + Serialize + for<'a> Deserialize<'a> + PartialEq + Debug + Clone>(
    old: T,
    new: T,
) {
    let diff = Diff::serializable(&old, &new);
    let json_diff = serde_json::to_string(&diff).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_diff);
    let mut target = old.clone();
    Apply::apply(&mut deserializer, &mut target).unwrap();
    assert_eq!(target, new);

    let bincode_diff = bincode::serialize(&diff).unwrap();
    let mut target = old;

    bincode::config()
        .deserialize_seed(Apply::deserializable(&mut target), &bincode_diff)
        .unwrap();
    assert_eq!(target, new);
}

fn partial<T: SerdeDiff + Serialize + for<'a> Deserialize<'a> + PartialEq + Debug + Clone>(
    old: T,
    new: T,
    target: T,
    expected: T,
) {
    let diff = Diff::serializable(&old, &new);
    let json_diff = serde_json::to_string(&diff).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_diff);
    let mut tmp_target = target.clone();
    Apply::apply(&mut deserializer, &mut tmp_target).unwrap();
    assert_eq!(tmp_target, expected);

    let bincode_diff = bincode::serialize(&diff).unwrap();
    let mut tmp_target = target;
    bincode::config()
        .deserialize_seed(Apply::deserializable(&mut tmp_target), &bincode_diff)
        .unwrap();
    assert_eq!(tmp_target, expected);
}

#[test]
fn test_option() {
    roundtrip(None::<TestStruct>, None);
    roundtrip(None, Some(TestStruct { a: 42, b: 12. }));
    roundtrip(Some(TestStruct { a: 42, b: 12. }), None);
    roundtrip(
        Some(TestStruct { a: 52, b: 32. }),
        Some(TestStruct { a: 42, b: 12. }),
    );

    partial(
        Some(TestStruct { a: 5, b: 2. }),
        Some(TestStruct { a: 8, b: 2. }),
        Some(TestStruct { a: 0, b: 4. }),
        Some(TestStruct { a: 8, b: 4. }),
    );
}

#[test]
fn test_array() {
    partial([0, 1, 2, 3], [0, 1, 9, 3], [4, 5, 6, 7], [4, 5, 9, 7]);

    partial(
        Some([0, 1, 2, 3]),
        Some([0, 1, 9, 3]),
        Some([4, 5, 6, 7]),
        Some([4, 5, 9, 7]),
    );

    partial(
        [
            None,
            Some(TestStruct { a: 5, b: 2. }),
            Some(TestStruct { a: 5, b: 2. }),
        ],
        [
            Some(TestStruct { a: 8, b: 2. }),
            Some(TestStruct { a: 8, b: 2. }),
            None,
        ],
        [None, Some(TestStruct { a: 0, b: 4. }), None],
        [
            Some(TestStruct { a: 8, b: 2. }),
            Some(TestStruct { a: 8, b: 4. }),
            None,
        ],
    );
}

#[test]
fn test_tuple() {
    roundtrip(
        (None::<TestStruct>, Some(TestStruct { a: 8, b: 2. })),
        (None, Some(TestStruct { a: 8, b: 2. })),
    );

    partial((0, 1, 2, 3), (0, 1, 9, 3), (4, 5, 6, 7), (4, 5, 9, 7));

    partial(
        Some((0, 1, 2, 3)),
        Some((0, 1, 9, 3)),
        Some((4, 5, 6, 7)),
        Some((4, 5, 9, 7)),
    );

    partial(
        (
            None,
            Some(TestStruct { a: 5, b: 2. }),
            Some(TestStruct { a: 5, b: 2. }),
        ),
        (
            Some(TestStruct { a: 8, b: 2. }),
            Some(TestStruct { a: 8, b: 2. }),
            None,
        ),
        (None, Some(TestStruct { a: 0, b: 4. }), None),
        (
            Some(TestStruct { a: 8, b: 2. }),
            Some(TestStruct { a: 8, b: 4. }),
            None,
        ),
    );
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(from = "MySimpleStruct", into = "MySimpleStruct")]
#[serde_diff(target = "MySimpleStruct")]
struct MyComplexStruct {
    // This field will be serialized
    a: u32,
    // This field will not be serialized, because it is not needed for <some reason>
    b: u32,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
#[serde(rename = "MyComplexStruct", default)]
struct MySimpleStruct {
    a: u32,
}

#[derive(SerdeDiff, Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
struct MyCowStruct<'x> {
    a: Cow<'x, MySimpleStruct>,
}

impl From<MySimpleStruct> for MyComplexStruct {
    fn from(my_simple_struct: MySimpleStruct) -> Self {
        MyComplexStruct {
            a: my_simple_struct.a,
            b: 0, // this value wasn't serialized, so we'll just default it to zero
        }
    }
}

impl Into<MySimpleStruct> for MyComplexStruct {
    fn into(self) -> MySimpleStruct {
        MySimpleStruct { a: self.a }
    }
}

fn targeted_roundtrip<T, U>(old: T, new: T, expected: T)
where
    T: SerdeDiff + Serialize + for<'a> Deserialize<'a> + PartialEq + Debug + Clone,
    U: SerdeDiff + Serialize + for<'a> Deserialize<'a>,
{
    let diff = Diff::serializable(&old, &new);
    let json_diff = serde_json::to_string(&diff).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_diff);
    let mut applied = old.clone();
    Apply::apply(&mut deserializer, &mut applied).unwrap();
    assert_eq!(applied, expected);

    let bincode_diff = bincode::serialize(&diff).unwrap();
    let mut applied = old;

    bincode::config()
        .deserialize_seed(Apply::deserializable(&mut applied), &bincode_diff)
        .unwrap();
    assert_eq!(applied, expected);
}

#[test]
fn test_targeted() {
    targeted_roundtrip::<MyComplexStruct, MySimpleStruct>(
        MyComplexStruct { a: 1, b: 777 },
        MyComplexStruct { a: 2, b: 999 },
        MyComplexStruct { a: 2, b: 0 },
    );
    targeted_roundtrip::<Option<MyComplexStruct>, Option<MySimpleStruct>>(
        Some(MyComplexStruct { a: 1, b: 777 }),
        Some(MyComplexStruct { a: 2, b: 999 }),
        Some(MyComplexStruct { a: 2, b: 0 }),
    );
}

#[test]
fn test_cow() {
    roundtrip(
        MyCowStruct {
            a: Cow::Owned(MySimpleStruct { a: 0 }),
        },
        MyCowStruct {
            a: Cow::Owned(MySimpleStruct { a: 10 }),
        },
    );
    let a = MySimpleStruct { a: 0 };
    let b = MySimpleStruct { a: 1 };
    roundtrip(
        MyCowStruct {
            a: Cow::Borrowed(&a),
        },
        MyCowStruct {
            a: Cow::Owned(MySimpleStruct { a: 10 }),
        },
    );
    roundtrip(
        MyCowStruct {
            a: Cow::Owned(MySimpleStruct { a: 0 }),
        },
        MyCowStruct {
            a: Cow::Borrowed(&b),
        },
    );
    roundtrip(
        MyCowStruct {
            a: Cow::Borrowed(&a),
        },
        MyCowStruct {
            a: Cow::Borrowed(&b),
        },
    );
}
