use serde::{Deserialize, Serialize};
use struct_diff::{Apply, Diff, SerdeDiffable};

#[derive(SerdeDiffable, Clone, PartialEq, Serialize, Deserialize, Debug)]
struct MyInnerStruct {
    string_list2: Vec<String>,
    //#[serde_diffable(skip)]
    x: f32,

    //#[serde_diffable(skip)]
    a_string: String,

    //#[serde_diffable(skip)]
    string_list: Vec<String>,
}

#[derive(SerdeDiffable, Serialize, Deserialize, Debug)]
struct MyStruct {
    //#[serde_diffable(skip)]
    a: f32,
    b: i32,
    //#[serde_diffable(skip)]
    s: String,
    //#[serde_diffable(skip)]
    c: MyInnerStruct,
}

fn main() {
    let (utf8_data, bincode_data) = {
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
        let mut vec = Vec::new();
        let mut c = std::io::Cursor::new(&mut vec);
        let mut serializer = serde_json::Serializer::new(&mut c);
        Diff::diff(&mut serializer, &old, &new).unwrap();

        let bincode_data = bincode::serialize(&Diff::serializable(&old, &new)).unwrap();
        // let bincode_data = vec![0];

        let utf8_data = String::from_utf8(vec).unwrap();

        println!("{}", utf8_data);
        println!("old: {:?} new {:?}", old, new);

        (utf8_data, bincode_data)
    };

    let mut target = MyStruct {
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
    // let c = std::io::Cursor::new(utf8_data.as_bytes());
    // let mut deserializer = serde_json::Deserializer::from_reader(c);
    // Apply::apply(&mut deserializer, &mut target).unwrap();

    bincode::config()
        .deserialize_seed(Apply::deserializable(&mut target), &bincode_data)
        .unwrap();
    println!("target {:?}", target);
    println!(
        "bincode size {} json size {}",
        bincode_data.len(),
        utf8_data.len()
    );
    // println!("str {:?}", std::str::from_utf8(&bincode_data).unwrap());
}
