# Serde path to error

[![Build Status](https://api.travis-ci.com/dtolnay/path-to-error.svg?branch=master)](https://travis-ci.com/dtolnay/path-to-error)
[![Latest Version](https://img.shields.io/crates/v/serde-path-to-error.svg)](https://crates.io/crates/serde_path_to_error)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/serde_path_to_error)

Find out the path at which a deserialization error occurred. This crate provides
a wrapper that works with any existing Serde `Deserializer` and exposes the
chain of field names leading to the error.

```toml
[dependencies]
serde = "1.0"
serde_path_to_error = "0.0"
```

```rust
use serde::Deserialize;
use std::collections::BTreeMap as Map;

#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Map<String, Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    version: String,
}

fn main() {
    let j = r#"{
        "name": "demo",
        "dependencies": {
            "serde": {
                "version": 1
            }
        }
    }"#;

    // Some Deserializer.
    let jd = &mut serde_json::Deserializer::from_str(j);

    let result: Result<Package, _> = serde_path_to_error::deserialize(jd);
    match result {
        Ok(_) => panic!("expected a type error"),
        Err(err) => {
            let path = err.path().to_string();
            assert_eq!(path, "dependencies.serde.version");
        }
    }
}
```

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
