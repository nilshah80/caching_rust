//! Redis value conversion helpers.
//!
//! Shared helpers for translating between Redis protocol values and JSON-facing
//! API payloads.

use serde_json::Value;

/// Convert a JSON value into a Redis argument string.
pub fn json_to_redis_arg(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(b) => {
            if *b {
                "1".to_string()
            } else {
                "0".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

/// Convert a Redis protocol value into JSON.
pub fn redis_value_to_json(value: redis::Value) -> Value {
    match value {
        redis::Value::Nil => Value::Null,
        redis::Value::Int(i) => Value::Number(i.into()),
        redis::Value::BulkString(bytes) => match String::from_utf8(bytes) {
            Ok(s) => Value::String(s),
            Err(_) => Value::Null,
        },
        redis::Value::Array(arr) => {
            Value::Array(arr.into_iter().map(redis_value_to_json).collect())
        }
        redis::Value::SimpleString(s) => Value::String(s),
        redis::Value::Okay => Value::String("OK".to_string()),
        redis::Value::Map(items) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in items {
                if let Value::String(key) = redis_value_to_json(k) {
                    obj.insert(key, redis_value_to_json(v));
                }
            }
            Value::Object(obj)
        }
        redis::Value::Attribute { data, .. } => redis_value_to_json(*data),
        redis::Value::Set(items) | redis::Value::Push { data: items, .. } => {
            Value::Array(items.into_iter().map(redis_value_to_json).collect())
        }
        redis::Value::Double(f) => serde_json::json!(f),
        redis::Value::Boolean(b) => serde_json::json!(b),
        redis::Value::VerbatimString { text, .. } => Value::String(text),
        redis::Value::BigNumber(n) => Value::String(n.to_string()),
        // ServerError type is not publicly constructible — only testable via real Redis errors
        redis::Value::ServerError(err) => Value::String(format!("{err:?}")),
        _ => Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_redis_arg_scalars() {
        assert_eq!(json_to_redis_arg(&Value::Null), "");
        assert_eq!(json_to_redis_arg(&serde_json::json!(true)), "1");
        assert_eq!(json_to_redis_arg(&serde_json::json!(false)), "0");
        assert_eq!(json_to_redis_arg(&serde_json::json!(42)), "42");
        assert_eq!(json_to_redis_arg(&serde_json::json!("hello")), "hello");
    }

    #[test]
    fn test_json_to_redis_arg_complex_values() {
        assert_eq!(json_to_redis_arg(&serde_json::json!([1, 2])), "[1,2]");
        assert_eq!(
            json_to_redis_arg(&serde_json::json!({"key": "value"})),
            "{\"key\":\"value\"}"
        );
    }

    #[test]
    fn test_redis_value_to_json_scalars() {
        assert_eq!(redis_value_to_json(redis::Value::Nil), Value::Null);
        assert_eq!(
            redis_value_to_json(redis::Value::Int(42)),
            serde_json::json!(42)
        );
        assert_eq!(
            redis_value_to_json(redis::Value::SimpleString("OK".to_string())),
            serde_json::json!("OK")
        );
        assert_eq!(
            redis_value_to_json(redis::Value::Boolean(true)),
            serde_json::json!(true)
        );
    }

    #[test]
    fn test_redis_value_to_json_okay() {
        assert_eq!(
            redis_value_to_json(redis::Value::Okay),
            serde_json::json!("OK")
        );
    }

    #[test]
    fn test_redis_value_to_json_double() {
        assert_eq!(
            redis_value_to_json(redis::Value::Double(3.14)),
            serde_json::json!(3.14)
        );
    }

    #[test]
    fn test_redis_value_to_json_boolean_false() {
        assert_eq!(
            redis_value_to_json(redis::Value::Boolean(false)),
            serde_json::json!(false)
        );
    }

    #[test]
    fn test_redis_value_to_json_set() {
        assert_eq!(
            redis_value_to_json(redis::Value::Set(vec![
                redis::Value::Int(1),
                redis::Value::Int(2),
            ])),
            serde_json::json!([1, 2])
        );
    }

    #[test]
    fn test_redis_value_to_json_verbatim_string() {
        assert_eq!(
            redis_value_to_json(redis::Value::VerbatimString {
                format: redis::VerbatimFormat::Text,
                text: "hello".to_string(),
            }),
            serde_json::json!("hello")
        );
    }

    #[test]
    fn test_redis_value_to_json_big_number() {
        use num_bigint::BigInt;
        assert_eq!(
            redis_value_to_json(redis::Value::BigNumber(BigInt::from(123456789i64))),
            serde_json::json!("123456789")
        );
    }

    // Note: redis::Value::ServerError cannot be tested in isolation because
    // the ServerError type is not re-exported from the redis crate.
    // Coverage for that branch is achieved via integration tests.

    #[test]
    fn test_redis_value_to_json_attribute() {
        let attr = redis::Value::Attribute {
            data: Box::new(redis::Value::Int(42)),
            attributes: vec![],
        };
        assert_eq!(redis_value_to_json(attr), serde_json::json!(42));
    }

    #[test]
    fn test_redis_value_to_json_bulk_string_invalid_utf8() {
        assert_eq!(
            redis_value_to_json(redis::Value::BulkString(vec![0xff, 0xfe])),
            Value::Null
        );
    }

    #[test]
    fn test_redis_value_to_json_map_non_string_key() {
        let map = redis::Value::Map(vec![(
            redis::Value::Int(1),
            redis::Value::BulkString(b"value".to_vec()),
        )]);
        // Non-string keys are skipped
        assert_eq!(redis_value_to_json(map), serde_json::json!({}));
    }

    #[test]
    fn test_redis_value_to_json_push() {
        let push = redis::Value::Push {
            kind: redis::PushKind::Message,
            data: vec![redis::Value::Int(1), redis::Value::Int(2)],
        };
        assert_eq!(redis_value_to_json(push), serde_json::json!([1, 2]));
    }

    #[test]
    fn test_redis_value_to_json_collections() {
        assert_eq!(
            redis_value_to_json(redis::Value::Array(vec![
                redis::Value::Int(1),
                redis::Value::BulkString(b"hello".to_vec())
            ])),
            serde_json::json!([1, "hello"])
        );

        assert_eq!(
            redis_value_to_json(redis::Value::Map(vec![(
                redis::Value::BulkString(b"name".to_vec()),
                redis::Value::BulkString(b"value".to_vec())
            )])),
            serde_json::json!({"name": "value"})
        );
    }
}
