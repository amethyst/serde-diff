use serde::{Deserialize, Serialize};
use serde_diff::{simple_serde_diff, Apply, Diff, SerdeDiff};

// Example of implementing diff support for trivial type. Must implement
// Serialize + Deserialize + PartialEq.
#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
struct SimpleWrapper(u32);
simple_serde_diff!(SimpleWrapper);

// Minimal example of implementing diff support for a struct
#[derive(SerdeDiff, Serialize, Deserialize, Clone, Debug)]
struct MySimpleStruct {
    val: u32,
}

// Example of a struct that does not implement SerdeDiff, but is still usable with `#[serde_diff(opaque)]`
#[derive(SerdeDiff, Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde_diff(opaque)]
struct OpaqueTest(i32);

// This struct is contained within MyStruct for a more complex example case
#[derive(SerdeDiff, Clone, Serialize, Deserialize, Debug)]
struct MyInnerStruct {
    x: f32,
    opaque: OpaqueTest,
    a_string: String,
    string_list: Vec<String>,
    string_list2: Vec<String>,
    nested_vec: Vec<MySimpleStruct>,
}

// This is a more complex struct containing another complex struct
#[derive(SerdeDiff, Clone, Serialize, Deserialize, Debug)]
struct MyStruct {
    a: f32,
    b: i32,
    s: String,
    c: MyInnerStruct,
    simple: SimpleWrapper,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create old state
    let old = MyStruct {
        a: 5.0,
        b: 31,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            opaque: OpaqueTest(9),
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str3".to_string()],
            string_list2: vec!["str6".to_string(), "str7".to_string()],
            nested_vec: vec![MySimpleStruct { val: 8 }],
        },
        simple: SimpleWrapper(10),
    };

    // Create new state
    let new = MyStruct {
        a: 3.0,
        b: 32,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 39.0,
            opaque: OpaqueTest(4),
            a_string: "my other string".to_string(),
            string_list: vec!["str1".to_string(), "str2".to_string(), "str3".to_string()],
            string_list2: vec!["str6".to_string()],
            nested_vec: vec![MySimpleStruct { val: 6 }],
        },
        simple: SimpleWrapper(4),
    };

    // Create a diff of the to structures. This just stores a reference to the old/new values
    // and doesn't walk any fields yet. The actual diff will be performed when this struct is
    // serialized
    let diff = Diff::serializable(&old, &new);

    // Serialize into a couple different formats
    let json_data = serde_json::to_string(&diff)?;
    let bincode_data = bincode::serialize(&diff)?;
    let msgpack_data = rmp_serde::to_vec_named(&diff)?;

    println!("{}", &json_data);
    // Create a struct to which we will apply a diff. This is a mix of old and new state from
    // the diff
    let target = MyStruct {
        a: 5.0,                    // old, 5.0 -> 3.0
        b: 31,                     // old, 31 -> 32
        s: "A string".to_string(), // unchanged
        c: MyInnerStruct {
            x: 40.0,                                                    // old, 40.0 -> 39.0
            opaque: OpaqueTest(9),                                      // old, 9 -> 4
            a_string: "my string".to_string(), // old, "my string" -> "my other string"
            string_list: vec!["str1".to_string(), "str5".to_string()], // does not match old or new, "str2" was added
            string_list2: vec!["str6".to_string(), "str7".to_string()], // old, "str7" was removed
            nested_vec: vec![MySimpleStruct { val: 3 }], // does not match old or new, 8 -> 6
        },
        simple: SimpleWrapper(10), // old, 10 -> 4
    };

    // Demonstrate applying the diff saved as json
    {
        let mut target = target.clone();
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target)?;
    }

    // Demonstrate applying the diff saved as bincode
    {
        let mut target = target.clone();
        bincode::config().deserialize_seed(Apply::deserializable(&mut target), &bincode_data)?;

        println!("diff {:#?} and {:#?}", old, new);
        println!("result {:#?}", target);
    }

    println!(
        "bincode size {} json size {} msgpack size {}",
        bincode_data.len(),
        json_data.len(),
        msgpack_data.len(),
    );
    Ok(())
}
