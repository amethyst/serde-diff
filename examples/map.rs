use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};
use std::collections::HashMap;

#[derive(SerdeDiff, Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct TestStruct {
    test: bool,
    //#[serde_diff(opaque)]
    map: HashMap<String, Vec<String>>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut empty = TestStruct::default();
    empty.test = true;

    let mut hello_world = TestStruct::default();
    hello_world
        .map
        .insert("hello".to_string(), vec!["world".to_string()]);

    let mut hi_world = TestStruct::default();
    hi_world
        .map
        .insert("hi".to_string(), vec!["world".to_string()]);

    let mut hi_world_and_planet = TestStruct::default();
    hi_world_and_planet.map.insert(
        "hi".to_string(),
        vec!["world".to_string(), "planet".to_string()],
    );

    let mut hi_planet = TestStruct::default();
    hi_planet
        .map
        .insert("hi".to_string(), vec!["planet".to_string()]);

    let mut hi_planet_hello_world = TestStruct::default();
    hi_planet_hello_world
        .map
        .insert("hi".to_string(), vec!["planet".to_string()]);
    hi_planet_hello_world
        .map
        .insert("hello".to_string(), vec!["world".to_string()]);

    let add_hello = serde_json::to_string(&Diff::serializable(&empty, &hello_world))?;
    let hello_to_hi = serde_json::to_string(&Diff::serializable(&hello_world, &hi_world))?;
    let add_planet = serde_json::to_string(&Diff::serializable(&hi_world, &hi_world_and_planet))?;
    let del_world = serde_json::to_string(&Diff::serializable(&hi_world_and_planet, &hi_planet))?;
    let no_change = serde_json::to_string(&Diff::serializable(&hi_planet, &hi_planet))?;
    let add_world = serde_json::to_string(&Diff::serializable(&hi_planet, &hi_planet_hello_world))?;

    let mut built = TestStruct::default();
    for (diff, after) in &[
        (add_hello, hello_world),
        (hello_to_hi, hi_world),
        (add_planet, hi_world_and_planet),
        (del_world, hi_planet.clone()),
        (no_change, hi_planet),
        (add_world, hi_planet_hello_world),
    ] {
        println!("{}", diff);

        let mut deserializer = serde_json::Deserializer::from_str(&diff);
        Apply::apply(&mut deserializer, &mut built)?;

        assert_eq!(after, &built);
    }
    Ok(())
}
