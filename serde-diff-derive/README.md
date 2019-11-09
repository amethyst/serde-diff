# serde-diff-derive
Derives the `serde_diff::SerdeDiff` trait.

# Usage
```
#[derive(SerdeDiff, Serialize, Deserialize)]
struct MyStruct { ... }
```