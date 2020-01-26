# serde-diff

A small helper that can
1. Serialize the fields that differ between two structs of the same type 
2. Apply previously serialized field differences to other structs.

The SerdeDiff trait impl can serialize field paths recursively, greatly reducing the amount of data that needs to be serialized when only a small part of a struct has changed. 

[![Build Status][build_img]][build_lnk] [![Crates.io][crates_img]][crates_lnk] [![Docs.rs][doc_img]][doc_lnk]

[build_img]: https://travis-ci.org/amethyst/serde-diff.svg
[build_lnk]: https://travis-ci.org/amethyst/serde-diff
[crates_img]: https://img.shields.io/crates/v/serde-diff.svg
[crates_lnk]: https://crates.io/crates/serde-diff
[doc_img]: https://docs.rs/serde-diff/badge.svg
[doc_lnk]: https://docs.rs/serde-diff

## Usage
On a struct:
```rust
#[derive(SerdeDiff, Serialize, Deserialize)]
```

Serialize & apply differences:

bincode
```rust
let bincode_data = bincode::serialize(&Diff::serializable(&old, &new)).unwrap();
bincode::config()
        .deserialize_seed(Apply::deserializable(&mut target), &bincode_data)
        .unwrap();
```
serde_json
```rust
        let json_data = serde_json::to_string(&Diff::serializable(&old, &new)).unwrap();
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target).unwrap();
```

## Built-in type support
- [x] Primitive types
- [x] std::time::{Duration, SystemTime}
- [x] IP addresses in std
- [x] Vec
- [x] HashMap (thanks @milkey-mouse)
- [x] BTreeMap (thanks @milkey-mouse)
- [x] Fixed-size arrays (thanks @Boscop)
- [x] Tuples (thanks @Boscop)

# Simple example

`Cargo.toml`
```toml
[dependencies]
serde-diff = "0.1.3"
serde = "1"
serde_json = "1" // all serde formats are supported, serde_json is shown in this example
```
`main.rs`
```rust
use serde_diff::{Apply, Diff, SerdeDiff};
use serde::{Serialize, Deserialize};
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq, Debug)]
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
        a: 8, // Differs from old.a, will be serialized
        b: 2.,
    };
    let mut target = TestStruct {
        a: 0,
        b: 4.,
    };
    let json_data = serde_json::to_string(&Diff::serializable(&old, &new)).unwrap();
    let mut deserializer = serde_json::Deserializer::from_str(&json_data);
    Apply::apply(&mut deserializer, &mut target).unwrap();


    let result = TestStruct {
        a: 8,
        b: 4.,
    };
    assert_eq!(result, target);
}
```

## Derive macro attributes
Opaque structs:
```rust
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
#[serde_diff(opaque)] // opaque structs are serialized as a unit and fields do not need to implement SerdeDiff
struct DoesNotRecurse {
    value: ExternalType, 
}
```

Opaque fields:
```rust
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
struct WrapperStruct {
    #[serde_diff(opaque)]
    value: ExternalType, // opaque fields only need to implement Serialize + Deserialize + PartialEq,
}
```

Skip fields:
```rust
#[derive(SerdeDiff, Serialize, Deserialize, PartialEq)]
struct WrapperStruct {
    #[serde_diff(skip)]
    value: ExternalType,
}
```

## Contribution

All contributions are assumed to be dual-licensed under MIT/Apache-2.

## License

Distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
