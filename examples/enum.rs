use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};

#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
enum TestEnum {
    Structish {x : u32, y: u32}, 
    Enumish (i32, i32, i32),
    Unitish,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    {
        let old = TestEnum::Structish { x: 5, y: 2 };
        let new = TestEnum::Structish {
            x: 8, // Differs from old.a, will be serialized
            y: 2,
        };
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        let mut target = TestEnum::Structish { x: 0, y: 4 };
        Apply::apply(&mut deserializer, &mut target)?;
        
        let result = TestEnum::Structish { x: 8, y: 4 };
        assert_eq!(result, target);
    }
    {
        let old = TestEnum::Structish { x: 5, y: 2 };
        let new = TestEnum::Structish {
            x: 8, // Differs from old.a, will be serialized
            y: 2,
        };
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        let mut target = TestEnum::Enumish ( 1, 2, 3);        
        Apply::apply(&mut deserializer, &mut target)?;
        /* we can't apply changes from one enum variant 
        to another, as there may be unfilled fields with no sane defaults,
        should there be an error? probably.*/
        let result = TestEnum::Enumish ( 1, 2, 3);
        assert_eq!(result, target);
    }
    {
        let old = TestEnum::Enumish (1, 2, 3);
        let new = TestEnum::Enumish (1, 10, 3);
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        let mut target = TestEnum::Enumish ( 4, 3, 2);        
        Apply::apply(&mut deserializer, &mut target)?;
        let result = TestEnum::Enumish (4, 10, 2); 
        assert_eq!(result, target);
    }
    {
        let old = TestEnum::Structish { x: 5, y: 2 };
        let new = TestEnum::Unitish;
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new))?;
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        let mut target = TestEnum::Enumish ( 1, 2, 3);        
        Apply::apply(&mut deserializer, &mut target)?;
        let result = TestEnum::Unitish;
        assert_eq!(result, target);
    }
    Ok(())
}
