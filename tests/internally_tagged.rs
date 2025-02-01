use serde_derive::Deserialize;
use serde_path_to_error;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum Inner {
    A(u32),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Outer {
    B { value: Inner },
}

#[test]
fn test_internally_tagged_path() {
    let json = r#"
    {
        "type": "B",
        "value": { "type": "A", "content": "500" }
    }"#;

    let json_deserializer = &mut serde_json::Deserializer::from_str(json);
    let result: Result<Outer, _> = serde_path_to_error::deserialize(json_deserializer);

    match result {
        Ok(_) => panic!("Expected error but got success"),
        Err(e) => {
            let path = e.path().to_string();
            assert_eq!(
                path, "value.content",
                "Path should point to the content field"
            );
        }
    }
}
