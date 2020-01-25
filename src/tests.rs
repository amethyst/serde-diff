use super::*;
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
