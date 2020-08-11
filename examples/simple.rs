use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};

#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
struct TestStruct {
    a: u32,
    b: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = TestStruct { a: 5, b: 2. };
    let new = TestStruct {
        a: 8, // Differs from old.a, will be serialized
        b: 2.,
    };
    let mut target = TestStruct { a: 0, b: 4. };
    let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
    println!("{}", json_data);
    let mut deserializer = serde_json::Deserializer::from_str(&json_data);
    Apply::apply(&mut deserializer, &mut target)?;

    let result = TestStruct { a: 8, b: 4. };
    assert_eq!(result, target);
    Ok(())
}
