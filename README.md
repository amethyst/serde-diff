# serde-diff

A small helper that can
1. Serialize the fields that differ between two structs of the same type 
2. Apply previously serialized field differences to other structs.

The SerdeDiffable can serialize field paths recursively, greatly reducing the amount of data that needs to be serialized.

[![Build Status](https://travis-ci.org/amethyst/serde-diff.svg?branch=master)](https://travis-ci.org/amethyst/serde-diff)

TODO: crates.io badge

## Status

Works for most basic use-cases. Includes derive macro, some standard library type implementations and deep serde integration. Supports diffing Vec<T>. Supports both text and binary serde formats.

## Usage
On a struct:
```
#[derive(SerdeDiffable, Serialize, Deserialize)]
```

Serialize & apply differences:

bincode
```
let bincode_data = bincode::serialize(&Diff::serializable(&old, &new)).unwrap();
bincode::config()
        .deserialize_seed(Apply::deserializable(&mut target), &bincode_data)
        .unwrap();
```
serde_json
```
        let mut deserializer = serde_json::Deserializer::from_str(&json_data);
        Apply::apply(&mut deserializer, &mut target).unwrap();
```

## Contribution

All contributions are assumed to be dual-licensed under MIT/Apache-2.

## License

Distributed under the terms of both the MIT license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
