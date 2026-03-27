//! Transaction Schemas
//!
//! Request and response types for transaction API endpoints.
//! Supports the single-request bundled transaction model for Redis MULTI/EXEC.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

// ========== Command Types ==========

/// Redis command to execute within a transaction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RedisCommand {
    // ========== String Commands ==========
    /// GET - Get the value of a key
    Get { key: String },
    /// SET - Set the value of a key with optional TTL
    Set {
        key: String,
        value: String,
        /// Optional TTL in seconds
        #[serde(skip_serializing_if = "Option::is_none")]
        ttl_seconds: Option<u64>,
    },
    /// INCR - Increment the integer value of a key by one
    Incr { key: String },
    /// INCRBY - Increment the integer value of a key by the given amount
    IncrBy { key: String, delta: i64 },
    /// DECR - Decrement the integer value of a key by one
    Decr { key: String },
    /// DECRBY - Decrement the integer value of a key by the given amount
    DecrBy { key: String, delta: i64 },
    /// APPEND - Append a value to a key
    Append { key: String, value: String },
    /// SETNX - Set the value of a key, only if the key does not exist
    SetNx { key: String, value: String },
    /// GETSET - Set the string value of a key and return its old value
    GetSet { key: String, value: String },
    /// MGET - Get the values of all specified keys
    MGet { keys: Vec<String> },
    /// MSET - Set multiple keys to multiple values
    MSet {
        /// Key-value pairs to set
        entries: Vec<KeyValue>,
    },

    // ========== Hash Commands ==========
    /// HGET - Get the value of a hash field
    HGet { key: String, field: String },
    /// HSET - Set the value of a hash field
    HSet {
        key: String,
        field: String,
        value: String,
    },
    /// HMSET - Set multiple hash fields to multiple values
    HMSet {
        key: String,
        /// Field-value pairs to set
        fields: Vec<FieldValue>,
    },
    /// HMGET - Get values of multiple hash fields
    HMGet { key: String, fields: Vec<String> },
    /// HINCRBY - Increment the integer value of a hash field
    HIncrBy {
        key: String,
        field: String,
        delta: i64,
    },
    /// HINCRBYFLOAT - Increment the float value of a hash field
    HIncrByFloat {
        key: String,
        field: String,
        delta: f64,
    },
    /// HDEL - Delete one or more hash fields
    HDel { key: String, fields: Vec<String> },
    /// HEXISTS - Check if a hash field exists
    HExists { key: String, field: String },
    /// HGETALL - Get all fields and values in a hash
    HGetAll { key: String },
    /// HKEYS - Get all field names in a hash
    HKeys { key: String },
    /// HVALS - Get all values in a hash
    HVals { key: String },
    /// HLEN - Get the number of fields in a hash
    HLen { key: String },
    /// HSETNX - Set the value of a hash field, only if it doesn't exist
    HSetNx {
        key: String,
        field: String,
        value: String,
    },

    // ========== List Commands ==========
    /// LPUSH - Insert all values at the head of the list
    LPush { key: String, values: Vec<String> },
    /// RPUSH - Insert all values at the tail of the list
    RPush { key: String, values: Vec<String> },
    /// LPOP - Remove and return elements from the head of the list
    LPop {
        key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<u32>,
    },
    /// RPOP - Remove and return elements from the tail of the list
    RPop {
        key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<u32>,
    },
    /// LLEN - Get the length of a list
    LLen { key: String },
    /// LINDEX - Get an element from a list by its index
    LIndex { key: String, index: i64 },
    /// LRANGE - Get a range of elements from a list
    LRange { key: String, start: i64, stop: i64 },
    /// LSET - Set the value of an element by its index
    LSet {
        key: String,
        index: i64,
        value: String,
    },
    /// LTRIM - Trim a list to the specified range
    LTrim { key: String, start: i64, stop: i64 },
    /// LREM - Remove elements from a list
    LRem {
        key: String,
        count: i64,
        value: String,
    },

    // ========== Set Commands ==========
    /// SADD - Add one or more members to a set
    SAdd { key: String, members: Vec<String> },
    /// SREM - Remove one or more members from a set
    SRem { key: String, members: Vec<String> },
    /// SISMEMBER - Check if a member exists in a set
    SIsMember { key: String, member: String },
    /// SMEMBERS - Get all members of a set
    SMembers { key: String },
    /// SCARD - Get the number of members in a set
    SCard { key: String },
    /// SPOP - Remove and return random members from a set
    SPop {
        key: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        count: Option<u32>,
    },
    /// SMOVE - Move a member from one set to another
    SMove {
        source: String,
        destination: String,
        member: String,
    },

    // ========== Sorted Set Commands ==========
    /// ZADD - Add one or more members to a sorted set with scores
    ZAdd {
        key: String,
        members: Vec<ScoredMember>,
    },
    /// ZREM - Remove one or more members from a sorted set
    ZRem { key: String, members: Vec<String> },
    /// ZINCRBY - Increment the score of a member in a sorted set
    ZIncrBy {
        key: String,
        delta: f64,
        member: String,
    },
    /// ZSCORE - Get the score of a member in a sorted set
    ZScore { key: String, member: String },
    /// ZRANK - Get the rank of a member in a sorted set (lowest to highest)
    ZRank { key: String, member: String },
    /// ZREVRANK - Get the rank of a member (highest to lowest)
    ZRevRank { key: String, member: String },
    /// ZCARD - Get the number of members in a sorted set
    ZCard { key: String },
    /// ZCOUNT - Count members in a score range
    ZCount {
        key: String,
        min: String,
        max: String,
    },
    /// ZRANGE - Return a range of members by index
    ZRange {
        key: String,
        start: i64,
        stop: i64,
        #[serde(default)]
        with_scores: bool,
    },
    /// ZREVRANGE - Return a range of members by index (highest to lowest)
    ZRevRange {
        key: String,
        start: i64,
        stop: i64,
        #[serde(default)]
        with_scores: bool,
    },

    // ========== Key Commands ==========
    /// DEL - Delete one or more keys
    Del { keys: Vec<String> },
    /// EXISTS - Check if keys exist
    Exists { keys: Vec<String> },
    /// EXPIRE - Set a timeout on a key (in seconds)
    Expire { key: String, seconds: u64 },
    /// PEXPIRE - Set a timeout on a key (in milliseconds)
    PExpire { key: String, milliseconds: u64 },
    /// TTL - Get the time to live for a key (in seconds)
    Ttl { key: String },
    /// PTTL - Get the time to live for a key (in milliseconds)
    PTtl { key: String },
    /// PERSIST - Remove the expiration from a key
    Persist { key: String },
    /// RENAME - Rename a key
    Rename { key: String, new_key: String },
    /// RENAMENX - Rename a key, only if the new key does not exist
    RenameNx { key: String, new_key: String },
    /// TYPE - Get the type of a key
    Type { key: String },
}

/// Key-value pair for MSET command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// Field-value pair for HMSET command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FieldValue {
    pub field: String,
    pub value: String,
}

/// Member with score for ZADD command
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScoredMember {
    pub score: f64,
    pub member: String,
}

// ========== Request Types ==========

/// Request to execute a transaction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct TransactionRequest {
    /// Keys to WATCH for optimistic locking (optional).
    /// If any watched key is modified by another client before EXEC,
    /// the transaction will abort and return an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watch_keys: Option<Vec<String>>,

    /// Commands to execute atomically within MULTI/EXEC.
    /// All commands are executed in order as a single atomic operation.
    #[validate(length(min = 1, message = "At least one command is required"))]
    pub commands: Vec<RedisCommand>,
}

/// Request for compare-and-set operation on a string key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct CompareAndSetRequest {
    /// The key to compare and set
    #[validate(length(min = 1, message = "Key is required"))]
    pub key: String,

    /// The expected current value
    pub expected_value: String,

    /// The new value to set if the current value matches
    pub new_value: String,
}

/// Request for compare-and-set operation on a hash field
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct HCompareAndSetRequest {
    /// The hash key
    #[validate(length(min = 1, message = "Key is required"))]
    pub key: String,

    /// The field within the hash
    #[validate(length(min = 1, message = "Field is required"))]
    pub field: String,

    /// The expected current value of the field
    pub expected_value: String,

    /// The new value to set if the current value matches
    pub new_value: String,
}

// ========== Response Types ==========

/// Result of a single command within a transaction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CommandResult {
    /// Index of the command in the request (0-based)
    pub index: usize,

    /// Whether this individual command succeeded
    pub success: bool,

    /// The result value (format depends on the command)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,

    /// Error message if the command failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Response from transaction execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionResponse {
    /// Whether the transaction completed successfully
    pub success: bool,

    /// Results from each command, in order
    pub results: Vec<CommandResult>,
}

/// Response from compare-and-set operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompareAndSetResponse {
    /// Whether the compare-and-set succeeded (value was updated)
    pub swapped: bool,

    /// The current value after the operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_value: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_redis_command_serialization() {
        let cmd = RedisCommand::Set {
            key: "test".to_string(),
            value: "hello".to_string(),
            ttl_seconds: Some(60),
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"SET\""));
        assert!(json.contains("\"key\":\"test\""));
        assert!(json.contains("\"value\":\"hello\""));
        assert!(json.contains("\"ttl_seconds\":60"));
    }

    #[test]
    fn test_redis_command_deserialization() {
        let json = r#"{"type":"GET","key":"mykey"}"#;
        let cmd: RedisCommand = serde_json::from_str(json).unwrap();
        match cmd {
            RedisCommand::Get { key } => assert_eq!(key, "mykey"),
            _ => panic!("Expected Get command"),
        }
    }

    #[test]
    fn test_transaction_request_deserialization() {
        let json = r#"{
            "watch_keys": ["counter"],
            "commands": [
                {"type": "GET", "key": "counter"},
                {"type": "INCR", "key": "counter"}
            ]
        }"#;
        let request: TransactionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.watch_keys.as_ref().unwrap().len(), 1);
        assert_eq!(request.commands.len(), 2);
    }

    #[test]
    fn test_compare_and_set_request() {
        let json = r#"{
            "key": "version",
            "expected_value": "1",
            "new_value": "2"
        }"#;
        let request: CompareAndSetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "version");
        assert_eq!(request.expected_value, "1");
        assert_eq!(request.new_value, "2");
    }

    #[test]
    fn test_hcompare_and_set_request() {
        let json = r#"{
            "key": "user:1",
            "field": "version",
            "expected_value": "1",
            "new_value": "2"
        }"#;
        let request: HCompareAndSetRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.key, "user:1");
        assert_eq!(request.field, "version");
    }

    #[test]
    fn test_transaction_response_serialization() {
        let response = TransactionResponse {
            success: true,
            results: vec![
                CommandResult {
                    index: 0,
                    success: true,
                    value: Some(json!("hello")),
                    error: None,
                },
                CommandResult {
                    index: 1,
                    success: true,
                    value: Some(json!(42)),
                    error: None,
                },
            ],
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"results\""));
    }

    #[test]
    fn test_zadd_command() {
        let cmd = RedisCommand::ZAdd {
            key: "leaderboard".to_string(),
            members: vec![
                ScoredMember {
                    score: 100.0,
                    member: "player1".to_string(),
                },
                ScoredMember {
                    score: 200.0,
                    member: "player2".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"Z_ADD\""));
        assert!(json.contains("player1"));
    }

    #[test]
    fn test_mset_command() {
        let cmd = RedisCommand::MSet {
            entries: vec![
                KeyValue {
                    key: "key1".to_string(),
                    value: "val1".to_string(),
                },
                KeyValue {
                    key: "key2".to_string(),
                    value: "val2".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"M_SET\""));
    }

    #[test]
    fn test_hash_commands() {
        let cmd = RedisCommand::HMSet {
            key: "user:1".to_string(),
            fields: vec![
                FieldValue {
                    field: "name".to_string(),
                    value: "John".to_string(),
                },
                FieldValue {
                    field: "age".to_string(),
                    value: "30".to_string(),
                },
            ],
        };
        let json = serde_json::to_string(&cmd).unwrap();
        assert!(json.contains("\"type\":\"H_M_SET\""));
        assert!(json.contains("\"field\":\"name\""));
    }
}
