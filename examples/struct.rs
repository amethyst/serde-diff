use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};

#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
struct TestStruct {
    a: u32,
    b: f64,
}
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
struct Test2Struct(u32, u32, u32);
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
struct Test3Struct;

#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
struct Test4Struct<T>
where T: SerdeDiff
{
    a: T,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        let old = TestStruct { a: 5, b: 2. };
        let new = TestStruct {
            a: 8, // Differs from old.a, will be serialized
            b: 2.,
        };
        let mut target = TestStruct { a: 0, b: 4. };
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target)?;

        let result = TestStruct { a: 8, b: 4. };
        assert_eq!(result, target);
    }
    {
        let old = Test2Struct (1, 2, 3);
        let new = Test2Struct (
            5, // Differs from old.0, will be serialized
            2,
            3
        );
        let mut target = Test2Struct (4,5,6);
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target)?;

        let result = Test2Struct ( 5,5,6 );
        assert_eq!(result, target);
    }
    {
        let old = Test3Struct;
        let new = Test3Struct;
        let mut target = Test3Struct;
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target)?;

        let result = Test3Struct;
        assert_eq!(result, target);
    }
    {
        let old = Test4Struct { a: 5};
        let new = Test4Struct { a: 7};
        let mut target = Test4Struct { a: 10};
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target)?;

        let result = Test4Struct { a: 7};
        assert_eq!(result, target);
    }
    Ok(())
}
