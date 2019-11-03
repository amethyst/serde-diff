use serde::{Deserialize, Serialize};
use struct_diff::{Apply, Diff, SerdeDiffable};

#[derive(SerdeDiffable, Clone, PartialEq, Serialize, Deserialize, Debug)]
struct MyInnerStruct {
    x: f32,
    a_string: String,
    string_list: Vec<String>,
    string_list2: Vec<String>,
}

#[derive(SerdeDiffable, Clone, Serialize, Deserialize, Debug)]
struct MyStruct {
    a: f32,
    b: i32,
    s: String,
    c: MyInnerStruct,
}

fn main() {
    // Create old state
    let old = MyStruct {
        a: 5.0,
        b: 31,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str3".to_string()],
            string_list2: vec!["str6".to_string(), "str7".to_string()],
        },
    };

    // Create new state
    let new = MyStruct {
        a: 3.0,
        b: 32,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 39.0,
            a_string: "my other string".to_string(),
            string_list: vec!["str1".to_string(), "str2".to_string(), "str3".to_string()],
            string_list2: vec!["str6".to_string()],
        },
    };
    let json_data = serde_json::to_string(&Diff::serializable(&old, &new)).unwrap();
    let bincode_data = bincode::serialize(&Diff::serializable(&old, &new)).unwrap();

    let target = MyStruct {
        a: 5.0,
        b: 31,
        s: "A string".to_string(),
        c: MyInnerStruct {
            x: 40.0,
            a_string: "my string".to_string(),
            string_list: vec!["str1".to_string(), "str5".to_string()],
            string_list2: vec!["str6".to_string(), "str7".to_string()],
        },
    };
    {
        let mut target = target.clone();
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target).unwrap();
    }

    {
        let mut target = target.clone();
        bincode::config()
            .deserialize_seed(Apply::deserializable(&mut target), &bincode_data)
            .unwrap();
        println!("target {:?}", target);
    }
    println!(
        "bincode size {} json size {}",
        bincode_data.len(),
        json_data.len()
    );
}
