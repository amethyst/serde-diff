use serde::{Deserialize, Serialize};
use serde_diff::{Apply, Diff, SerdeDiff};

#[derive(SerdeDiff, Serialize, Deserialize, Debug, PartialEq, Clone)]
enum Value {
    Str(String),
    Int(i32),
}

impl From<&str> for Value {
    fn from(other: &str) -> Self {
        Self::Str(other.into())
    }
}

#[derive(SerdeDiff, Serialize, Deserialize, Debug, Default, PartialEq, Clone)]
struct TestStruct {
    test: bool,
    //#[serde_diff(opaque)]
    list: Vec<Value>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut empty = TestStruct::default();
    empty.test = true;

    let mut a = TestStruct::default();
    a.list.push("a".into());

    let mut abc = TestStruct::default();
    abc.list.push("a".into());
    abc.list.push("b".into());
    abc.list.push("c".into());

    let mut cba = TestStruct::default();
    cba.list.push("c".into());
    cba.list.push("b".into());
    cba.list.push("a".into());

    let mut c = TestStruct::default();
    c.list.push("c".into());

    let add_a = serde_json::to_string(&Diff::serializable(&empty, &a))?;
    let add_b = serde_json::to_string(&Diff::serializable(&a, &abc))?;
    let del_a = serde_json::to_string(&Diff::serializable(&abc, &cba))?;
    let rep_b_c = serde_json::to_string(&Diff::serializable(&cba, &c))?;
    let no_change = serde_json::to_string(&Diff::serializable(&c, &c))?;

    let mut built = TestStruct::default();
    for (diff, after) in &[
        (add_a, a),
        (add_b, abc),
        (del_a, cba),
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
