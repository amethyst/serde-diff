use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};
use std::collections::HashSet;

#[derive(SerdeDiff, Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct TestStruct {
    test: bool,
    //#[serde_diff(opaque)]
    set: HashSet<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut empty = TestStruct::default();
    empty.test = true;

    let mut a = TestStruct::default();
    a.set.insert("a".to_string());

    let mut ab = TestStruct::default();
    ab.set.insert("a".to_string());
    ab.set.insert("b".to_string());

    let mut b = TestStruct::default();
    b.set.insert("b".to_string());

    let mut c = TestStruct::default();
    c.set.insert("c".to_string());

    let add_a = serde_json::to_string(&Diff::serializable(&empty, &a))?;
    let add_b = serde_json::to_string(&Diff::serializable(&a, &ab))?;
    let del_a = serde_json::to_string(&Diff::serializable(&ab, &b))?;
    let rep_b_c = serde_json::to_string(&Diff::serializable(&b, &c))?;
    let no_change = serde_json::to_string(&Diff::serializable(&c, &c))?;

    let mut built = TestStruct::default();
    for (diff, after) in &[
        (add_a, a),
        (add_b, ab),
        (del_a, b),
        (rep_b_c, c.clone()),
        (no_change, c),
    ] {
        println!("{}", diff);

        let mut deserializer = serde_json::Deserializer::from_str(&diff);
        Apply::apply(&mut deserializer, &mut built)?;

        assert_eq!(after, &built);
    }
    Ok(())
}
