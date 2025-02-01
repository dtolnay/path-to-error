#![allow(dead_code)]
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum TestStruct {
    A(u32),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Wrapper {
    B { value: TestStruct },
}

#[test]
fn test_internally_tagged_error_path() {
    let failing_json = r#"
    {
        "type": "B",
        "value": { "type": "A", "content": "500" }
    }"#;

    let json_deserializer = &mut serde_json::Deserializer::from_str(failing_json);
    let result: Result<Wrapper, _> = serde_path_to_error::deserialize(json_deserializer);

    match result {
        Ok(_) => panic!("Expected error but got success"),
        Err(e) => {
            let path = e.path().to_string();
            let err = e.into_inner();
            assert_eq!(
                path, "value.content",
                "Error path should point to content field"
            );
            assert!(
                err.to_string()
                    .contains("invalid type: string \"500\", expected u32"),
                "Error message should indicate type mismatch"
            );
        }
    }
}
