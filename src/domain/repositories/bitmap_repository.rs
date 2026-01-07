//! Bitmap Repository Trait
//!
//! Abstract interface for Redis bitmap operations.

use async_trait::async_trait;

use crate::domain::errors::CacheError;

/// Bitwise operation type for BITOP command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitOperation {
    /// AND - result bit is 1 only if all corresponding bits are 1
    And,
    /// OR - result bit is 1 if any corresponding bit is 1
    Or,
    /// XOR - result bit is 1 if odd number of corresponding bits are 1
    Xor,
    /// NOT - invert all bits (only works with single source key)
    Not,
}

impl BitOperation {
    /// Get the Redis command string for this operation
    pub fn as_str(&self) -> &'static str {
        match self {
            BitOperation::And => "AND",
            BitOperation::Or => "OR",
            BitOperation::Xor => "XOR",
            BitOperation::Not => "NOT",
        }
    }
}

/// Overflow handling for BITFIELD operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BitfieldOverflow {
    /// Wrap around on overflow (default)
    #[default]
    Wrap,
    /// Use saturated arithmetic (clamp to min/max)
    Sat,
    /// Fail on overflow (return nil)
    Fail,
}

impl BitfieldOverflow {
    /// Get the Redis command string for this overflow mode
    pub fn as_str(&self) -> &'static str {
        match self {
            BitfieldOverflow::Wrap => "WRAP",
            BitfieldOverflow::Sat => "SAT",
            BitfieldOverflow::Fail => "FAIL",
        }
    }
}

/// Encoding type for BITFIELD operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitfieldEncoding {
    /// Signed integer (e.g., "i8", "i16", "i32", "i64")
    Signed(u8),
    /// Unsigned integer (e.g., "u8", "u16", "u32", "u63")
    Unsigned(u8),
}

impl BitfieldEncoding {
    /// Get the Redis encoding string (e.g., "i8", "u16")
    pub fn as_str(&self) -> String {
        match self {
            BitfieldEncoding::Signed(bits) => format!("i{}", bits),
            BitfieldEncoding::Unsigned(bits) => format!("u{}", bits),
        }
    }
}

/// A single BITFIELD subcommand
#[derive(Debug, Clone)]
pub enum BitfieldCommand {
    /// GET <encoding> <offset> - Get the value at the specified offset
    Get {
        encoding: BitfieldEncoding,
        offset: i64,
    },
    /// SET <encoding> <offset> <value> - Set the value at the specified offset
    Set {
        encoding: BitfieldEncoding,
        offset: i64,
        value: i64,
    },
    /// INCRBY <encoding> <offset> <increment> - Increment the value at the specified offset
    IncrBy {
        encoding: BitfieldEncoding,
        offset: i64,
        increment: i64,
    },
    /// OVERFLOW <mode> - Set overflow handling for subsequent operations
    Overflow(BitfieldOverflow),
}

/// Result of a BITFIELD operation
#[derive(Debug, Clone)]
pub struct BitfieldResult {
    /// Results from each subcommand (None for OVERFLOW commands or failed operations)
    pub values: Vec<Option<i64>>,
}

/// Repository trait for Redis bitmap operations
#[async_trait]
pub trait BitMapRepository: Send + Sync {
    // ========== Basic bit operations ==========

    /// SETBIT - Set the bit at offset in the string value stored at key
    /// Returns the original bit value at that offset (0 or 1)
    async fn setbit(&self, key: &str, offset: u64, value: bool) -> Result<i64, CacheError>;

    /// GETBIT - Get the bit at offset in the string value stored at key
    /// Returns 0 or 1
    async fn getbit(&self, key: &str, offset: u64) -> Result<i64, CacheError>;

    // ========== Counting operations ==========

    /// BITCOUNT - Count set bits (population counting) in a string
    /// If start/end are specified, they refer to byte positions
    async fn bitcount(
        &self,
        key: &str,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError>;

    /// BITPOS - Find first bit set to 0 or 1 in a string
    /// Returns the position of the first bit set to the specified value, or -1 if not found
    async fn bitpos(
        &self,
        key: &str,
        bit: bool,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError>;

    // ========== Bitwise operations ==========

    /// BITOP - Perform bitwise operation between strings
    /// Returns the size of the resulting string (bytes)
    async fn bitop(
        &self,
        operation: BitOperation,
        dest_key: &str,
        keys: &[String],
    ) -> Result<i64, CacheError>;

    // ========== BITFIELD operations ==========

    /// BITFIELD - Perform arbitrary bitfield operations on a string
    /// Executes multiple subcommands and returns results
    async fn bitfield(
        &self,
        key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError>;

    /// BITFIELD_RO - Read-only variant of BITFIELD (only GET operations)
    /// Safer for use with read replicas
    async fn bitfield_ro(
        &self,
        key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_operation_as_str() {
        assert_eq!(BitOperation::And.as_str(), "AND");
        assert_eq!(BitOperation::Or.as_str(), "OR");
        assert_eq!(BitOperation::Xor.as_str(), "XOR");
        assert_eq!(BitOperation::Not.as_str(), "NOT");
    }

    #[test]
    fn test_bitfield_overflow_as_str() {
        assert_eq!(BitfieldOverflow::Wrap.as_str(), "WRAP");
        assert_eq!(BitfieldOverflow::Sat.as_str(), "SAT");
        assert_eq!(BitfieldOverflow::Fail.as_str(), "FAIL");
    }

    #[test]
    fn test_bitfield_encoding_as_str() {
        assert_eq!(BitfieldEncoding::Signed(8).as_str(), "i8");
        assert_eq!(BitfieldEncoding::Unsigned(16).as_str(), "u16");
        assert_eq!(BitfieldEncoding::Signed(32).as_str(), "i32");
        assert_eq!(BitfieldEncoding::Unsigned(63).as_str(), "u63");
    }

    #[test]
    fn test_bitfield_result() {
        let result = BitfieldResult {
            values: vec![Some(42), None, Some(-1)],
        };
        assert_eq!(result.values.len(), 3);
        assert_eq!(result.values[0], Some(42));
        assert_eq!(result.values[1], None);
        assert_eq!(result.values[2], Some(-1));
    }
}
