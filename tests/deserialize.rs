#![allow(clippy::unreadable_literal, dead_code)]

use serde::Deserialize;
use serde_derive::Deserialize;
use std::collections::BTreeMap as Map;
use std::fmt::Debug;

fn test<'de, T>(json: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    let de = &mut serde_json::Deserializer::from_str(json);
    let result: Result<T, _> = serde_path_to_error::deserialize(de);
    let path = result.unwrap_err().path().to_string();
    assert_eq!(path, expected);
}

#[test]
fn test_struct() {
    #[derive(Deserialize, Debug)]
    struct Package {
        name: String,
        dependencies: Map<String, Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "name": "demo",
        "dependencies": {
            "serde": {
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependencies.serde.version");
}

#[test]
fn test_vec() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependencies: Vec<Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        name: String,
        version: String,
    }

    let j = r#"{
        "dependencies": [
            {
                "name": "serde",
                "version": "1.0"
            },
            {
                "name": "serde_json",
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependencies[1].version");
}

#[test]
fn test_option() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Option<Dependency>,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "version": 1
        }
    }"#;

    test::<Package>(j, "dependency.version");
}

#[test]
fn test_struct_variant() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    enum Dependency {
        Struct { version: String },
    }

    let j = r#"{
        "dependency": {
            "Struct": {
                "version": 1
            }
        }
    }"#;

    test::<Package>(j, "dependency.Struct.version");
}

#[test]
fn test_tuple_variant() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    enum Dependency {
        Tuple(String, String),
    }

    let j = r#"{
        "dependency": {
            "Tuple": ["serde", 1]
        }
    }"#;

    test::<Package>(j, "dependency.Tuple[1]");
}

#[test]
fn test_nested_tagged_enums() {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", content = "content")]
    enum TestStruct {
        A(u32),
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    enum Wrapper {
        B { value: TestStruct },
    }

    let j = r#"{
        "type": "B",
        "value": { "type": "A", "content": "500" }
    }"#;

    test::<Wrapper>(j, "value.content");
}

#[test]
fn test_internally_tagged_enum_nested() {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    enum InnerValue {
        Number { data: u32 },
    }

    #[derive(Deserialize, Debug)]
    #[serde(tag = "kind")]
    enum OuterValue {
        Container { inner: InnerValue },
    }

    #[derive(Deserialize, Debug)]
    struct Root {
        outer: OuterValue,
    }

    let json = r#"{
        "outer": {
            "kind": "Container",
            "inner": {
                "type": "Number",
                "data": "not_a_number"
            }
        }
    }"#;

    test::<Root>(json, "outer.inner.data");
}

#[test]
fn test_internally_tagged_enum_in_array() {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    enum Value {
        Number { content: u32 },
    }

    #[derive(Deserialize, Debug)]
    struct Container {
        values: Vec<Value>,
    }

    let json = r#"{
        "values": [
            { "type": "Number", "content": 123 },
            { "type": "Number", "content": "not_a_number" }
        ]
    }"#;

    test::<Container>(json, "values[1].content");
}

#[test]
fn test_internally_tagged_enum_missing_tag() {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type")]
    enum Value {
        Number { data: u32 },
    }

    #[derive(Deserialize, Debug)]
    struct Container {
        value: Value,
    }

    let json = r#"{
        "value": {
            "data": 123
        }
    }"#;

    test::<Container>(json, "value");
}

#[test]
fn test_adjacently_tagged_enum_struct_variant() {
    #[derive(Deserialize, Debug)]
    #[serde(tag = "type", content = "data")]
    enum Value {
        Complex { field1: String, field2: u32 },
    }

    #[derive(Deserialize, Debug)]
    struct Container {
        value: Value,
    }

    let json = r#"{
        "value": {
            "type": "Complex",
            "data": {
                "field1": "valid",
                "field2": "not_a_number"
            }
        }
    }"#;

    test::<Container>(json, "value.data.field2");
}

#[test]
fn test_unknown_field() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "version": "1.0",
            "name": "serde"
        }
    }"#;

    test::<Package>(j, "dependency.name");
}

#[test]
fn test_invalid_length() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency(String, String);

    let j = r#"{
        "dependency": ["serde"]
    }"#;

    test::<Package>(j, "dependency");
}

#[test]
fn test_syntax_error() {
    #[derive(Deserialize, Debug)]
    struct Package {
        dependency: Dependency,
    }

    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "dependency": {
            "error": *
    }"#;

    test::<Package>(j, "dependency.error");
}

#[test]
fn test_u128() {
    #[derive(Deserialize, Debug)]
    struct Container {
        n: u128,
    }

    let j = r#"{
        "n": 130033514578017493995102500318550798591
    }"#;

    let de = &mut serde_json::Deserializer::from_str(j);
    let container: Container = serde_path_to_error::deserialize(de).expect("failed to deserialize");

    assert_eq!(container.n, 130033514578017493995102500318550798591u128);
}

#[test]
fn test_map_nonstring_key() {
    #[derive(Deserialize, Debug)]
    struct Dependency {
        version: String,
    }

    let j = r#"{
        "100": {
            "version": false
        }
    }"#;

    test::<Map<i32, Dependency>>(j, "100.version");
}
