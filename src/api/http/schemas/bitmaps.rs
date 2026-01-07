//! Bitmap Schemas
//!
//! Request and response types for bitmap API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::domain::repositories::{BitfieldEncoding, BitfieldOverflow};

// ========== Request Types ==========

/// Request to set a bit value
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct BitSetRequest {
    /// The bit value to set (true = 1, false = 0)
    pub value: bool,
}

/// Query parameters for BITCOUNT
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct BitCountQuery {
    /// Start position (byte index by default, or bit index if use_bit=true)
    pub start: Option<i64>,
    /// End position (byte index by default, or bit index if use_bit=true)
    pub end: Option<i64>,
    /// If true, start/end refer to bit positions; if false (default), byte positions
    #[serde(default)]
    pub use_bit: bool,
}

/// Query parameters for BITPOS
#[derive(Debug, Serialize, Deserialize, ToSchema, Default)]
pub struct BitPosQuery {
    /// The bit value to search for (true = 1, false = 0)
    pub bit: bool,
    /// Start position (byte index by default, or bit index if use_bit=true)
    pub start: Option<i64>,
    /// End position (byte index by default, or bit index if use_bit=true)
    pub end: Option<i64>,
    /// If true, start/end refer to bit positions; if false (default), byte positions
    #[serde(default)]
    pub use_bit: bool,
}

/// Bitwise operation type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum BitOpType {
    /// AND - result bit is 1 only if all corresponding bits are 1
    And,
    /// OR - result bit is 1 if any corresponding bit is 1
    Or,
    /// XOR - result bit is 1 if odd number of corresponding bits are 1
    Xor,
    /// NOT - invert all bits (only works with single source key)
    Not,
}

impl From<BitOpType> for crate::domain::repositories::BitOperation {
    fn from(op: BitOpType) -> Self {
        match op {
            BitOpType::And => crate::domain::repositories::BitOperation::And,
            BitOpType::Or => crate::domain::repositories::BitOperation::Or,
            BitOpType::Xor => crate::domain::repositories::BitOperation::Xor,
            BitOpType::Not => crate::domain::repositories::BitOperation::Not,
        }
    }
}

/// Request for BITOP operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BitOpRequest {
    /// The bitwise operation to perform
    pub operation: BitOpType,
    /// Destination key to store the result
    #[validate(length(min = 1, message = "Destination key is required"))]
    pub dest_key: String,
    /// Source keys to operate on
    #[validate(length(min = 1, message = "At least one source key is required"))]
    pub keys: Vec<String>,
}

/// Encoding type for BITFIELD operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", content = "bits")]
pub enum BitfieldEncodingSchema {
    /// Signed integer (e.g., 8 for i8, 16 for i16)
    #[serde(rename = "signed")]
    Signed(u8),
    /// Unsigned integer (e.g., 8 for u8, 16 for u16)
    #[serde(rename = "unsigned")]
    Unsigned(u8),
}

impl From<BitfieldEncodingSchema> for BitfieldEncoding {
    fn from(schema: BitfieldEncodingSchema) -> Self {
        match schema {
            BitfieldEncodingSchema::Signed(bits) => BitfieldEncoding::Signed(bits),
            BitfieldEncodingSchema::Unsigned(bits) => BitfieldEncoding::Unsigned(bits),
        }
    }
}

/// Overflow handling mode for BITFIELD operations
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum BitfieldOverflowSchema {
    /// Wrap around on overflow (default)
    #[default]
    Wrap,
    /// Use saturated arithmetic (clamp to min/max)
    Sat,
    /// Fail on overflow (return nil)
    Fail,
}

impl From<BitfieldOverflowSchema> for BitfieldOverflow {
    fn from(schema: BitfieldOverflowSchema) -> Self {
        match schema {
            BitfieldOverflowSchema::Wrap => BitfieldOverflow::Wrap,
            BitfieldOverflowSchema::Sat => BitfieldOverflow::Sat,
            BitfieldOverflowSchema::Fail => BitfieldOverflow::Fail,
        }
    }
}

/// A single BITFIELD subcommand
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "command", rename_all = "UPPERCASE")]
pub enum BitfieldCommandSchema {
    /// GET - Read a value at the specified offset
    Get {
        /// Encoding type (signed or unsigned with bit width)
        encoding: BitfieldEncodingSchema,
        /// Bit offset (can be prefixed with # for field index)
        offset: i64,
    },
    /// SET - Set a value at the specified offset
    Set {
        /// Encoding type (signed or unsigned with bit width)
        encoding: BitfieldEncodingSchema,
        /// Bit offset (can be prefixed with # for field index)
        offset: i64,
        /// Value to set
        value: i64,
    },
    /// INCRBY - Increment a value at the specified offset
    #[serde(rename = "INCRBY")]
    IncrBy {
        /// Encoding type (signed or unsigned with bit width)
        encoding: BitfieldEncodingSchema,
        /// Bit offset (can be prefixed with # for field index)
        offset: i64,
        /// Increment value (can be negative)
        increment: i64,
    },
    /// OVERFLOW - Set overflow handling for subsequent operations
    Overflow {
        /// Overflow handling mode
        mode: BitfieldOverflowSchema,
    },
}

impl From<BitfieldCommandSchema> for crate::domain::repositories::BitfieldCommand {
    fn from(schema: BitfieldCommandSchema) -> Self {
        match schema {
            BitfieldCommandSchema::Get { encoding, offset } => {
                crate::domain::repositories::BitfieldCommand::Get {
                    encoding: encoding.into(),
                    offset,
                }
            }
            BitfieldCommandSchema::Set {
                encoding,
                offset,
                value,
            } => crate::domain::repositories::BitfieldCommand::Set {
                encoding: encoding.into(),
                offset,
                value,
            },
            BitfieldCommandSchema::IncrBy {
                encoding,
                offset,
                increment,
            } => crate::domain::repositories::BitfieldCommand::IncrBy {
                encoding: encoding.into(),
                offset,
                increment,
            },
            BitfieldCommandSchema::Overflow { mode } => {
                crate::domain::repositories::BitfieldCommand::Overflow(mode.into())
            }
        }
    }
}

/// Request for BITFIELD operation
#[derive(Debug, Serialize, Deserialize, ToSchema, Validate)]
pub struct BitfieldRequest {
    /// List of subcommands to execute
    #[validate(length(min = 1, message = "At least one command is required"))]
    pub commands: Vec<BitfieldCommandSchema>,
}

// ========== Response Types ==========

/// Response from SETBIT
#[derive(Debug, Serialize, ToSchema)]
pub struct BitSetResponse {
    /// The original bit value at the offset (0 or 1)
    pub original_value: i64,
}

/// Response from GETBIT
#[derive(Debug, Serialize, ToSchema)]
pub struct BitGetResponse {
    /// The bit value at the offset (0 or 1)
    pub value: i64,
}

/// Response from BITCOUNT
#[derive(Debug, Serialize, ToSchema)]
pub struct BitCountResponse {
    /// Number of bits set to 1
    pub count: i64,
}

/// Response from BITPOS
#[derive(Debug, Serialize, ToSchema)]
pub struct BitPosResponse {
    /// Position of the first bit with the specified value (-1 if not found)
    pub position: i64,
}

/// Response from BITOP
#[derive(Debug, Serialize, ToSchema)]
pub struct BitOpResponse {
    /// Size of the resulting string in bytes
    pub size: i64,
}

/// Response from BITFIELD/BITFIELD_RO
#[derive(Debug, Serialize, ToSchema)]
pub struct BitfieldResponse {
    /// Results from each subcommand (None for OVERFLOW commands or failed operations)
    pub values: Vec<Option<i64>>,
}

impl From<crate::domain::repositories::BitfieldResult> for BitfieldResponse {
    fn from(result: crate::domain::repositories::BitfieldResult) -> Self {
        Self {
            values: result.values,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_set_request() {
        let req: BitSetRequest = serde_json::from_str(r#"{"value": true}"#).unwrap();
        assert!(req.value);

        let req: BitSetRequest = serde_json::from_str(r#"{"value": false}"#).unwrap();
        assert!(!req.value);
    }

    #[test]
    fn test_bit_count_query_defaults() {
        let query: BitCountQuery = serde_json::from_str(r#"{}"#).unwrap();
        assert!(query.start.is_none());
        assert!(query.end.is_none());
        assert!(!query.use_bit);
    }

    #[test]
    fn test_bit_count_query_with_range() {
        let query: BitCountQuery =
            serde_json::from_str(r#"{"start": 0, "end": 10, "use_bit": true}"#).unwrap();
        assert_eq!(query.start, Some(0));
        assert_eq!(query.end, Some(10));
        assert!(query.use_bit);
    }

    #[test]
    fn test_bit_op_request() {
        let req: BitOpRequest = serde_json::from_str(
            r#"{"operation": "AND", "dest_key": "result", "keys": ["key1", "key2"]}"#,
        )
        .unwrap();
        assert_eq!(req.operation, BitOpType::And);
        assert_eq!(req.dest_key, "result");
        assert_eq!(req.keys.len(), 2);
    }

    #[test]
    fn test_bit_op_types() {
        let and: BitOpType = serde_json::from_str(r#""AND""#).unwrap();
        assert_eq!(and, BitOpType::And);

        let or: BitOpType = serde_json::from_str(r#""OR""#).unwrap();
        assert_eq!(or, BitOpType::Or);

        let xor: BitOpType = serde_json::from_str(r#""XOR""#).unwrap();
        assert_eq!(xor, BitOpType::Xor);

        let not: BitOpType = serde_json::from_str(r#""NOT""#).unwrap();
        assert_eq!(not, BitOpType::Not);
    }

    #[test]
    fn test_bitfield_encoding() {
        let signed: BitfieldEncodingSchema =
            serde_json::from_str(r#"{"type": "signed", "bits": 8}"#).unwrap();
        assert!(matches!(signed, BitfieldEncodingSchema::Signed(8)));

        let unsigned: BitfieldEncodingSchema =
            serde_json::from_str(r#"{"type": "unsigned", "bits": 16}"#).unwrap();
        assert!(matches!(unsigned, BitfieldEncodingSchema::Unsigned(16)));
    }

    #[test]
    fn test_bitfield_command_get() {
        let cmd: BitfieldCommandSchema = serde_json::from_str(
            r#"{"command": "GET", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0}"#,
        )
        .unwrap();
        assert!(matches!(cmd, BitfieldCommandSchema::Get { .. }));
    }

    #[test]
    fn test_bitfield_command_set() {
        let cmd: BitfieldCommandSchema = serde_json::from_str(
            r#"{"command": "SET", "encoding": {"type": "signed", "bits": 16}, "offset": 8, "value": 42}"#,
        )
        .unwrap();
        assert!(matches!(cmd, BitfieldCommandSchema::Set { value: 42, .. }));
    }

    #[test]
    fn test_bitfield_command_incrby() {
        let cmd: BitfieldCommandSchema = serde_json::from_str(
            r#"{"command": "INCRBY", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0, "increment": 10}"#,
        )
        .unwrap();
        assert!(matches!(
            cmd,
            BitfieldCommandSchema::IncrBy { increment: 10, .. }
        ));
    }

    #[test]
    fn test_bitfield_command_overflow() {
        let cmd: BitfieldCommandSchema =
            serde_json::from_str(r#"{"command": "OVERFLOW", "mode": "SAT"}"#).unwrap();
        assert!(matches!(
            cmd,
            BitfieldCommandSchema::Overflow {
                mode: BitfieldOverflowSchema::Sat
            }
        ));
    }

    #[test]
    fn test_bitfield_request() {
        let req: BitfieldRequest = serde_json::from_str(
            r#"{"commands": [
                {"command": "SET", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0, "value": 100},
                {"command": "GET", "encoding": {"type": "unsigned", "bits": 8}, "offset": 0}
            ]}"#,
        )
        .unwrap();
        assert_eq!(req.commands.len(), 2);
    }

    #[test]
    fn test_bit_op_conversion() {
        assert!(matches!(
            crate::domain::repositories::BitOperation::from(BitOpType::And),
            crate::domain::repositories::BitOperation::And
        ));
        assert!(matches!(
            crate::domain::repositories::BitOperation::from(BitOpType::Or),
            crate::domain::repositories::BitOperation::Or
        ));
        assert!(matches!(
            crate::domain::repositories::BitOperation::from(BitOpType::Xor),
            crate::domain::repositories::BitOperation::Xor
        ));
        assert!(matches!(
            crate::domain::repositories::BitOperation::from(BitOpType::Not),
            crate::domain::repositories::BitOperation::Not
        ));
    }

    #[test]
    fn test_bitfield_response() {
        let result = crate::domain::repositories::BitfieldResult {
            values: vec![Some(42), None, Some(-1)],
        };
        let response = BitfieldResponse::from(result);
        assert_eq!(response.values.len(), 3);
        assert_eq!(response.values[0], Some(42));
        assert_eq!(response.values[1], None);
        assert_eq!(response.values[2], Some(-1));
    }
}
