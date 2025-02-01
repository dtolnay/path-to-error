#![allow(dead_code)]
use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
struct Function {
    #[serde(rename = "type")]
    fn_type: String,
    variants: Variants,
}

#[derive(Debug, Deserialize)]
struct Variants {
    #[serde(rename = "my_variant")]
    variant: Variant,
}

#[derive(Debug, Deserialize)]
struct Variant {
    #[serde(rename = "type")]
    variant_type: String,
    model: String,
}

#[test]
fn test_missing_field_path() {
    let json = r#"
    {
        "type": "chat",
        "variants": {
            "my_variant": {
                "model": "anthropic::claude-3-haiku-20240307"
            }
        }
    }"#;

    let json_deserializer = &mut serde_json::Deserializer::from_str(json);
    let result: Result<Function, _> = serde_path_to_error::deserialize(json_deserializer);

    match result {
        Ok(_) => panic!("Expected error but got success"),
        Err(e) => {
            let path = e.path().to_string();
            let err = e.into_inner();
            assert_eq!(
                path, "variants.my_variant.type",
                "Error path should point to missing type field"
            );
            assert!(
                err.to_string().contains("missing field"),
                "Error message should indicate missing field"
            );
        }
    }
}
