use actors::Actor;
use objects::Object;

pub mod activities;
pub mod actors;
pub mod cache;
pub mod coalesced_activity;
pub mod follows;
pub mod instances;
pub mod mls_group_conversations;
pub mod mls_key_packages;
pub mod notifications;
pub mod objects;
pub mod profiles;
pub mod unprocessable;
pub mod vault;
use serde_json::Value;

#[derive(Clone, Debug)]
pub enum Tombstone {
    Actor(Actor),
    Object(Object),
}

pub fn parameter_generator() -> impl FnMut() -> String {
    let mut counter = 1;
    move || {
        let param = format!("${counter}");
        counter += 1;
        param
    }
}

pub fn from_serde<T: serde::de::DeserializeOwned>(object: Value) -> Option<T> {
    match serde_json::from_value(object.clone()) {
        Ok(result) => Some(result),
        Err(e) => {
            log::debug!(
                "from_serde deserialization failed for type {}: {e}",
                std::any::type_name::<T>()
            );
            log::trace!("Raw JSON that failed to deserialize: {object}");
            None
        }
    }
}

pub fn from_serde_option<T: serde::de::DeserializeOwned>(object: Option<Value>) -> Option<T> {
    object.and_then(|o| from_serde(o))
}

pub struct OffsetPaging {
    pub page: u32,
    pub limit: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[test]
    fn test_parameter_generator() {
        let mut gen = parameter_generator();
        let p1 = gen(); // Call and store result
        assert_eq!(p1, "$1");
        let p2 = gen(); // Call and store result
        assert_eq!(p2, "$2");
        let p3 = gen(); // Call and store result
        assert_eq!(p3, "$3");
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct TestStruct {
        field1: String,
        field2: i32,
    }

    #[test]
    fn test_from_serde_success() {
        let value = json!({
            "field1": "hello",
            "field2": 42
        });
        let expected = TestStruct {
            field1: "hello".to_string(),
            field2: 42,
        };
        let result: Option<TestStruct> = from_serde(value);
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_from_serde_failure() {
        let value = json!({
            "field1": "hello",
            "field_wrong": 42 // Incorrect field name
        });
        let result: Option<TestStruct> = from_serde(value);
        assert_eq!(result, None);

        let value_wrong_type = json!({
            "field1": 123, // Incorrect type
            "field2": 42
        });
        let result_wrong_type: Option<TestStruct> = from_serde(value_wrong_type);
        assert_eq!(result_wrong_type, None);
    }

    #[test]
    fn test_from_serde_option_some_success() {
        let value = json!({
            "field1": "world",
            "field2": 99
        });
        let expected = TestStruct {
            field1: "world".to_string(),
            field2: 99,
        };
        let result: Option<TestStruct> = from_serde_option(Some(value));
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_from_serde_option_some_failure() {
        let value = json!({
            "field1": "world",
            "field_wrong": 99 // Incorrect field name
        });
        let result: Option<TestStruct> = from_serde_option(Some(value));
        assert_eq!(result, None);

        let value_wrong_type = json!({
            "field1": true, // Incorrect type
            "field2": 99
        });
        let result_wrong_type: Option<TestStruct> = from_serde_option(Some(value_wrong_type));
        assert_eq!(result_wrong_type, None);
    }

    #[test]
    fn test_from_serde_option_none() {
        let result: Option<TestStruct> = from_serde_option(None);
        assert_eq!(result, None);
    }
}
