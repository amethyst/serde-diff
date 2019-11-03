# struct-diff

A small helper to diff structs of the same type and apply those differences to other structs

TODO: Build Status and crates.io badges

## Status

Works for most basic use-cases. Includes derive macro, some standard library type implementations and deep serde integration. Supports both text and binary serde formats.

## Usage
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
