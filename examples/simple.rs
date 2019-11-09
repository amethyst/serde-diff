use serde_diff::{Apply, Diff, SerdeDiff};
use serde::{Serialize, Deserialize};
#[derive(SerdeDiff, Serialize, Deserialize, Debug)]
struct TestStruct {
    a: u32,
    b: f64,
}

fn main() {
    let old = TestStruct {
        a: 5,
        b: 2.,
    };
    let new = TestStruct {
        a: 8,
        b: 2.,
    };
    let mut target = TestStruct {
        a: 0,
        b: 2.,
    };
    let json_data = serde_json::to_string(&Diff::serializable(&old, &new)).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_data);
    Apply::apply(&mut deserializer, &mut target).unwrap();

    println!("Result: {:?}", target);
}

