//! Redis Bitmap Repository Implementation
//!
//! Concrete implementation of BitMapRepository using Redis.

use async_trait::async_trait;
use redis::Value;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    BitMapRepository, BitOperation, BitfieldCommand, BitfieldResult,
};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of BitMapRepository
#[derive(Clone)]
pub struct RedisBitMapRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisBitMapRepository {
    /// Create a new RedisBitMapRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl BitMapRepository for RedisBitMapRepository {
    async fn setbit(&self, key: &str, offset: u64, value: bool) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let bit_value: i64 = if value { 1 } else { 0 };
        let result: i64 = redis::cmd("SETBIT")
            .arg(key)
            .arg(offset)
            .arg(bit_value)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn getbit(&self, key: &str, offset: u64) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: i64 = redis::cmd("GETBIT")
            .arg(key)
            .arg(offset)
            .query_async(&mut *conn)
            .await?;
        Ok(result)
    }

    async fn bitcount(
        &self,
        key: &str,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BITCOUNT");
        cmd.arg(key);

        if let (Some(s), Some(e)) = (start, end) {
            cmd.arg(s).arg(e);
            if use_bit_index {
                cmd.arg("BIT");
            }
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn bitpos(
        &self,
        key: &str,
        bit: bool,
        start: Option<i64>,
        end: Option<i64>,
        use_bit_index: bool,
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BITPOS");
        let bit_value: i64 = if bit { 1 } else { 0 };
        cmd.arg(key).arg(bit_value);

        if let Some(s) = start {
            cmd.arg(s);
            if let Some(e) = end {
                cmd.arg(e);
                if use_bit_index {
                    cmd.arg("BIT");
                }
            }
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn bitop(
        &self,
        operation: BitOperation,
        dest_key: &str,
        keys: &[String],
    ) -> Result<i64, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BITOP");
        cmd.arg(operation.as_str()).arg(dest_key);

        for key in keys {
            cmd.arg(key);
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(result)
    }

    async fn bitfield(
        &self,
        key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BITFIELD");
        cmd.arg(key);

        for command in commands {
            match command {
                BitfieldCommand::Get { encoding, offset } => {
                    cmd.arg("GET").arg(encoding.as_str()).arg(*offset);
                }
                BitfieldCommand::Set {
                    encoding,
                    offset,
                    value,
                } => {
                    cmd.arg("SET")
                        .arg(encoding.as_str())
                        .arg(*offset)
                        .arg(*value);
                }
                BitfieldCommand::IncrBy {
                    encoding,
                    offset,
                    increment,
                } => {
                    cmd.arg("INCRBY")
                        .arg(encoding.as_str())
                        .arg(*offset)
                        .arg(*increment);
                }
                BitfieldCommand::Overflow(overflow) => {
                    cmd.arg("OVERFLOW").arg(overflow.as_str());
                }
            }
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;
        let values: Vec<Option<i64>> = result
            .into_iter()
            .map(|v| match v {
                Value::Int(i) => Some(i),
                Value::Nil => None,
                _ => None,
            })
            .collect();

        Ok(BitfieldResult { values })
    }

    async fn bitfield_ro(
        &self,
        key: &str,
        commands: &[BitfieldCommand],
    ) -> Result<BitfieldResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("BITFIELD_RO");
        cmd.arg(key);

        // BITFIELD_RO only supports GET operations
        for command in commands {
            if let BitfieldCommand::Get { encoding, offset } = command {
                cmd.arg("GET").arg(encoding.as_str()).arg(*offset);
            }
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;
        let values: Vec<Option<i64>> = result
            .into_iter()
            .map(|v| match v {
                Value::Int(i) => Some(i),
                Value::Nil => None,
                _ => None,
            })
            .collect();

        Ok(BitfieldResult { values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repositories::{BitfieldEncoding, BitfieldOverflow};

    #[test]
    fn test_bitfield_command_encoding() {
        let get_cmd = BitfieldCommand::Get {
            encoding: BitfieldEncoding::Unsigned(8),
            offset: 0,
        };
        assert!(matches!(get_cmd, BitfieldCommand::Get { .. }));

        let set_cmd = BitfieldCommand::Set {
            encoding: BitfieldEncoding::Signed(16),
            offset: 100,
            value: 42,
        };
        assert!(matches!(set_cmd, BitfieldCommand::Set { .. }));

        let incr_cmd = BitfieldCommand::IncrBy {
            encoding: BitfieldEncoding::Unsigned(32),
            offset: 0,
            increment: 10,
        };
        assert!(matches!(incr_cmd, BitfieldCommand::IncrBy { .. }));

        let overflow_cmd = BitfieldCommand::Overflow(BitfieldOverflow::Sat);
        assert!(matches!(overflow_cmd, BitfieldCommand::Overflow(_)));
    }
}
