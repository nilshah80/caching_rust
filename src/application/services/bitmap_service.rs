//! Bitmap Service
//!
//! Business logic for Redis bitmap operations.

use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    BitMapRepository, BitOperation, BitfieldCommand, BitfieldEncoding, BitfieldResult,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisBitMapRepository;

/// Service for bitmap operations
pub struct BitMapService {
    repository: Arc<dyn BitMapRepository>,
}

impl BitMapService {
    /// Create a new BitMapService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisBitMapRepository::new(pool)))
    }

    /// Create a new BitMapService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn BitMapRepository>) -> Self {
        Self { repository }
    }

    // ========== Basic bit operations ==========

    /// SETBIT - Set the bit at offset in the string value stored at key
    /// Returns the original bit value at that offset (0 or 1)
    pub async fn setbit(&self, key: &str, offset: u64, value: bool) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        self.repository.setbit(key, offset, value).await
    }

    /// GETBIT - Get the bit at offset in the string value stored at key
    /// Returns 0 or 1
    pub async fn getbit(&self, key: &str, offset: u64) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        self.repository.getbit(key, offset).await
    }

    // ========== Counting operations ==========

    /// BITCOUNT - Count set bits (population counting) in a string
    /// If start/end are specified, they refer to byte positions (or bit positions if use_bit_index=true)
    pub async fn bitcount(
        &self,
        key: &str,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        // Validate that start and end are both provided or both absent
        if start.is_some() != end.is_some() {
            return Err(CacheError::InvalidInput(
                "Start and end must both be provided or both absent".to_string(),
            ));
        }
        self.repository
            .bitcount(key, start, end, use_bit_index)
            .await
    }

    /// BITPOS - Find first bit set to 0 or 1 in a string
    /// Returns the position of the first bit set to the specified value, or -1 if not found
    pub async fn bitpos(
        &self,
        key: &str,
        bit: bool,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        // For BITPOS, end can only be specified if start is specified
        if end.is_some() && start.is_none() {
            return Err(CacheError::InvalidInput(
                "End can only be specified if start is also specified".to_string(),
            ));
        }
        self.repository
            .bitpos(key, bit, start, end, use_bit_index)
            .await
    }

    // ========== Bitwise operations ==========

    /// BITOP - Perform bitwise operation between strings
    /// Returns the size of the resulting string (bytes)
    pub async fn bitop(
        &self,
        operation: BitOperation,
        dest_key: &str,
        keys: Vec<String>,
    ) -> Result<i64, CacheError> {
        if dest_key.is_empty() {
            return Err(CacheError::InvalidInput(
                "Destination key cannot be empty".to_string(),
            ));
        }
        if keys.is_empty() {
            return Err(CacheError::InvalidInput("Keys cannot be empty".to_string()));
        }
        // NOT operation only works with a single key
        if operation == BitOperation::Not && keys.len() != 1 {
            return Err(CacheError::InvalidInput(
                "NOT operation requires exactly one source key".to_string(),
            ));
        }
        self.repository.bitop(operation, dest_key, &keys).await
    }

    // ========== BITFIELD operations ==========

    /// BITFIELD - Perform arbitrary bitfield operations on a string
    /// Executes multiple subcommands and returns results
    pub async fn bitfield(
        &self,
        key: &str,
        commands: Vec<BitfieldCommand>,
    ) -> Result<BitfieldResult, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if commands.is_empty() {
            return Err(CacheError::InvalidInput(
                "Commands cannot be empty".to_string(),
            ));
        }
        // Validate encoding bits (max 64 for signed, 63 for unsigned)
        for cmd in &commands {
            Self::validate_bitfield_command(cmd)?
        }
        self.repository.bitfield(key, &commands).await
    }

    /// BITFIELD_RO - Read-only variant of BITFIELD (only GET operations)
    /// Safer for use with read replicas
    pub async fn bitfield_ro(
        &self,
        key: &str,
        commands: Vec<BitfieldCommand>,
    ) -> Result<BitfieldResult, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if commands.is_empty() {
            return Err(CacheError::InvalidInput(
                "Commands cannot be empty".to_string(),
            ));
        }
        // Validate that only GET commands are used
        for cmd in &commands {
            if !matches!(cmd, BitfieldCommand::Get { .. }) {
                return Err(CacheError::InvalidInput(
                    "BITFIELD_RO only supports GET operations".to_string(),
                ));
            }
            Self::validate_bitfield_command(cmd)?
        }
        self.repository.bitfield_ro(key, &commands).await
    }

    /// Validate a bitfield command encoding
    fn validate_bitfield_command(cmd: &BitfieldCommand) -> Result<(), CacheError> {
        let encoding = match cmd {
            BitfieldCommand::Get { encoding, .. } => Some(encoding),
            BitfieldCommand::Set { encoding, .. } => Some(encoding),
            BitfieldCommand::IncrBy { encoding, .. } => Some(encoding),
            BitfieldCommand::Overflow(_) => None,
        };

        if let Some(enc) = encoding {
            match enc {
                BitfieldEncoding::Signed(bits) => {
                    if *bits == 0 || *bits > 64 {
                        return Err(CacheError::InvalidInput(
                            "Signed encoding bits must be between 1 and 64".to_string(),
                        ));
                    }
                }
                BitfieldEncoding::Unsigned(bits) => {
                    if *bits == 0 || *bits > 63 {
                        return Err(CacheError::InvalidInput(
                            "Unsigned encoding bits must be between 1 and 63".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::BitfieldOverflow;
    use crate::test_support::MockBitMapRepository;

    #[tokio::test]
    async fn test_setbit_empty_key() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let err = service.setbit("", 0, true).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_getbit_empty_key() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let err = service.getbit("", 0).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitcount_validation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Empty key
        let err = service.bitcount("", None, None, false).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Mismatched start/end
        let err = service
            .bitcount("key", Some(0), None, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitpos_validation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Empty key
        let err = service
            .bitpos("", true, None, None, false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // End without start
        let err = service
            .bitpos("key", true, None, Some(10), false)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitop_validation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Empty dest key
        let err = service
            .bitop(BitOperation::And, "", vec!["key1".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Empty keys
        let err = service
            .bitop(BitOperation::And, "dest", vec![])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // NOT with multiple keys
        let err = service
            .bitop(
                BitOperation::Not,
                "dest",
                vec!["key1".to_string(), "key2".to_string()],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitfield_validation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Empty key
        let err = service.bitfield("", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Empty commands
        let err = service.bitfield("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Invalid encoding (0 bits)
        let err = service
            .bitfield(
                "key",
                vec![BitfieldCommand::Get {
                    encoding: BitfieldEncoding::Unsigned(0),
                    offset: 0,
                }],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Invalid encoding (> 64 bits for signed)
        let err = service
            .bitfield(
                "key",
                vec![BitfieldCommand::Get {
                    encoding: BitfieldEncoding::Signed(65),
                    offset: 0,
                }],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Invalid encoding (> 63 bits for unsigned)
        let err = service
            .bitfield(
                "key",
                vec![BitfieldCommand::Get {
                    encoding: BitfieldEncoding::Unsigned(64),
                    offset: 0,
                }],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitfield_ro_validation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Empty key
        let err = service
            .bitfield_ro(
                "",
                vec![BitfieldCommand::Get {
                    encoding: BitfieldEncoding::Unsigned(8),
                    offset: 0,
                }],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Empty commands
        let err = service.bitfield_ro("key", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        // Non-GET command
        let err = service
            .bitfield_ro(
                "key",
                vec![BitfieldCommand::Set {
                    encoding: BitfieldEncoding::Unsigned(8),
                    offset: 0,
                    value: 42,
                }],
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_bitfield_incrby_valid_encoding() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let result = service
            .bitfield(
                "key",
                vec![BitfieldCommand::IncrBy {
                    encoding: BitfieldEncoding::Unsigned(8),
                    offset: 0,
                    increment: 1,
                }],
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bitfield_overflow_command() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let result = service
            .bitfield(
                "key",
                vec![BitfieldCommand::Overflow(BitfieldOverflow::Wrap)],
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_setbit_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let result = service.setbit("mybitmap", 7, true).await.unwrap();
        assert_eq!(result, 0); // Original value was 0
    }

    #[tokio::test]
    async fn test_getbit_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Set a bit first
        service.setbit("mybitmap", 7, true).await.unwrap();

        let result = service.getbit("mybitmap", 7).await.unwrap();
        assert_eq!(result, 1);

        let result = service.getbit("mybitmap", 0).await.unwrap();
        assert_eq!(result, 0);
    }

    #[tokio::test]
    async fn test_bitcount_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        // Set some bits
        service.setbit("mybitmap", 0, true).await.unwrap();
        service.setbit("mybitmap", 1, true).await.unwrap();
        service.setbit("mybitmap", 7, true).await.unwrap();

        let count = service
            .bitcount("mybitmap", None, None, false)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_bitpos_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        service.setbit("mybitmap", 7, true).await.unwrap();

        let pos = service
            .bitpos("mybitmap", true, None, None, false)
            .await
            .unwrap();
        assert_eq!(pos, 7);
    }

    #[tokio::test]
    async fn test_bitop_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        service.setbit("key1", 0, true).await.unwrap();
        service.setbit("key2", 0, true).await.unwrap();

        let size = service
            .bitop(
                BitOperation::And,
                "dest",
                vec!["key1".to_string(), "key2".to_string()],
            )
            .await
            .unwrap();
        assert!(size > 0);
    }

    #[tokio::test]
    async fn test_bitfield_operation() {
        let repo = Arc::new(MockBitMapRepository::new());
        let service = BitMapService::new_with_repository(repo);

        let result = service
            .bitfield(
                "mybitmap",
                vec![
                    BitfieldCommand::Set {
                        encoding: BitfieldEncoding::Unsigned(8),
                        offset: 0,
                        value: 42,
                    },
                    BitfieldCommand::Get {
                        encoding: BitfieldEncoding::Unsigned(8),
                        offset: 0,
                    },
                ],
            )
            .await
            .unwrap();

        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_bitmap_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = BitMapService::new(pool);
    }
}
