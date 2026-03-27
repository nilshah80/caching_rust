//! Redis Geo Repository Implementation
//!
//! Concrete implementation of GeoRepository using Redis.

use async_trait::async_trait;
use redis::Value;
use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    GeoAddOptions, GeoAddResult, GeoMember, GeoPosition, GeoRepository, GeoSearchCenter,
    GeoSearchOptions, GeoSearchResult, GeoSearchShape, GeoSearchStoreResult, GeoSortOrder, GeoUnit,
};
use crate::infrastructure::redis::connection::InstrumentedPool;

/// Redis implementation of GeoRepository
#[derive(Clone)]
pub struct RedisGeoRepository {
    pool: Arc<InstrumentedPool>,
}

impl RedisGeoRepository {
    /// Create a new RedisGeoRepository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self { pool }
    }

    /// Parse a geo search result from Redis Value
    fn parse_search_result(
        value: Value,
        with_dist: bool,
        with_coord: bool,
        with_hash: bool,
    ) -> Option<GeoSearchResult> {
        match value {
            // Simple case: just the member name
            Value::BulkString(bytes) => Some(GeoSearchResult {
                member: String::from_utf8_lossy(&bytes).to_string(),
                distance: None,
                position: None,
                geohash: None,
            }),
            // Complex case: array with member and optional fields
            Value::Array(arr) => {
                if arr.is_empty() {
                    return None;
                }

                let member = match &arr[0] {
                    Value::BulkString(bytes) => String::from_utf8_lossy(bytes).to_string(),
                    _ => return None,
                };

                let mut idx = 1;
                let mut distance = None;
                let mut geohash = None;
                let mut position = None;

                // Parse optional fields in order: dist, hash, coord
                if with_dist && idx < arr.len() {
                    distance = Self::parse_f64(&arr[idx]);
                    idx += 1;
                }

                if with_hash && idx < arr.len() {
                    geohash = match &arr[idx] {
                        Value::Int(i) => Some(*i),
                        _ => None,
                    };
                    idx += 1;
                }

                if with_coord
                    && idx < arr.len()
                    && let Value::Array(coords) = &arr[idx]
                    && coords.len() >= 2
                {
                    let lon = Self::parse_f64(&coords[0]);
                    let lat = Self::parse_f64(&coords[1]);
                    if let (Some(lon), Some(lat)) = (lon, lat) {
                        position = Some(GeoPosition::new(lon, lat));
                    }
                }

                Some(GeoSearchResult {
                    member,
                    distance,
                    position,
                    geohash,
                })
            }
            _ => None,
        }
    }

    /// Parse a float from Redis Value
    fn parse_f64(value: &Value) -> Option<f64> {
        match value {
            Value::BulkString(bytes) => String::from_utf8_lossy(bytes).parse().ok(),
            Value::Double(f) => Some(*f),
            _ => None,
        }
    }
}

#[async_trait]
impl GeoRepository for RedisGeoRepository {
    async fn geo_add(
        &self,
        key: &str,
        members: &[GeoMember],
        options: GeoAddOptions,
    ) -> Result<GeoAddResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEOADD");
        cmd.arg(key);

        // Add options
        if options.nx {
            cmd.arg("NX");
        } else if options.xx {
            cmd.arg("XX");
        }
        if options.ch {
            cmd.arg("CH");
        }

        // Add members
        for member in members {
            cmd.arg(member.position.longitude)
                .arg(member.position.latitude)
                .arg(&member.member);
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;
        Ok(GeoAddResult {
            added: result,
            changed: if options.ch { Some(result) } else { None },
        })
    }

    async fn geo_pos(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<GeoPosition>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEOPOS");
        cmd.arg(key);

        for member in members {
            cmd.arg(member);
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;

        let positions: Vec<Option<GeoPosition>> = result
            .into_iter()
            .map(|v| match v {
                Value::Array(coords) if coords.len() >= 2 => {
                    let lon = Self::parse_f64(&coords[0]);
                    let lat = Self::parse_f64(&coords[1]);
                    match (lon, lat) {
                        (Some(lon), Some(lat)) => Some(GeoPosition::new(lon, lat)),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();

        Ok(positions)
    }

    async fn geo_dist(
        &self,
        key: &str,
        member1: &str,
        member2: &str,
        unit: GeoUnit,
    ) -> Result<Option<f64>, CacheError> {
        let mut conn = self.pool.get().await?;
        let result: Value = redis::cmd("GEODIST")
            .arg(key)
            .arg(member1)
            .arg(member2)
            .arg(unit.as_str())
            .query_async(&mut *conn)
            .await?;

        Ok(Self::parse_f64(&result))
    }

    async fn geo_hash(
        &self,
        key: &str,
        members: &[String],
    ) -> Result<Vec<Option<String>>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEOHASH");
        cmd.arg(key);

        for member in members {
            cmd.arg(member);
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;

        let hashes: Vec<Option<String>> = result
            .into_iter()
            .map(|v| match v {
                Value::BulkString(bytes) => Some(String::from_utf8_lossy(&bytes).to_string()),
                _ => None,
            })
            .collect();

        Ok(hashes)
    }

    async fn geo_search(
        &self,
        key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEOSEARCH");
        cmd.arg(key);

        // Add center
        match &center {
            GeoSearchCenter::FromMember(member) => {
                cmd.arg("FROMMEMBER").arg(member);
            }
            GeoSearchCenter::FromLonLat(pos) => {
                cmd.arg("FROMLONLAT").arg(pos.longitude).arg(pos.latitude);
            }
        }

        // Add shape
        match &shape {
            GeoSearchShape::ByRadius { radius, unit } => {
                cmd.arg("BYRADIUS").arg(*radius).arg(unit.as_str());
            }
            GeoSearchShape::ByBox {
                width,
                height,
                unit,
            } => {
                cmd.arg("BYBOX").arg(*width).arg(*height).arg(unit.as_str());
            }
        }

        // Add sort order
        if let Some(sort) = &options.sort {
            match sort {
                GeoSortOrder::Asc => cmd.arg("ASC"),
                GeoSortOrder::Desc => cmd.arg("DESC"),
            };
        }

        // Add count
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
            if options.count_any {
                cmd.arg("ANY");
            }
        }

        // Add optional flags
        if options.with_coord {
            cmd.arg("WITHCOORD");
        }
        if options.with_dist {
            cmd.arg("WITHDIST");
        }
        if options.with_hash {
            cmd.arg("WITHHASH");
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;

        let results: Vec<GeoSearchResult> = result
            .into_iter()
            .filter_map(|v| {
                Self::parse_search_result(
                    v,
                    options.with_dist,
                    options.with_coord,
                    options.with_hash,
                )
            })
            .collect();

        Ok(results)
    }

    async fn geo_search_store(
        &self,
        dest_key: &str,
        source_key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
        store_dist: bool,
    ) -> Result<GeoSearchStoreResult, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEOSEARCHSTORE");
        cmd.arg(dest_key).arg(source_key);

        // Add center
        match &center {
            GeoSearchCenter::FromMember(member) => {
                cmd.arg("FROMMEMBER").arg(member);
            }
            GeoSearchCenter::FromLonLat(pos) => {
                cmd.arg("FROMLONLAT").arg(pos.longitude).arg(pos.latitude);
            }
        }

        // Add shape
        match &shape {
            GeoSearchShape::ByRadius { radius, unit } => {
                cmd.arg("BYRADIUS").arg(*radius).arg(unit.as_str());
            }
            GeoSearchShape::ByBox {
                width,
                height,
                unit,
            } => {
                cmd.arg("BYBOX").arg(*width).arg(*height).arg(unit.as_str());
            }
        }

        // Add sort order
        if let Some(sort) = &options.sort {
            match sort {
                GeoSortOrder::Asc => cmd.arg("ASC"),
                GeoSortOrder::Desc => cmd.arg("DESC"),
            };
        }

        // Add count
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
            if options.count_any {
                cmd.arg("ANY");
            }
        }

        // Add store distance flag
        if store_dist {
            cmd.arg("STOREDIST");
        }

        let result: i64 = cmd.query_async(&mut *conn).await?;

        Ok(GeoSearchStoreResult { stored: result })
    }

    async fn geo_radius(
        &self,
        key: &str,
        position: GeoPosition,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEORADIUS");
        cmd.arg(key)
            .arg(position.longitude)
            .arg(position.latitude)
            .arg(radius)
            .arg(unit.as_str());

        // Add optional flags
        if options.with_coord {
            cmd.arg("WITHCOORD");
        }
        if options.with_dist {
            cmd.arg("WITHDIST");
        }
        if options.with_hash {
            cmd.arg("WITHHASH");
        }

        // Add sort order
        if let Some(sort) = &options.sort {
            match sort {
                GeoSortOrder::Asc => cmd.arg("ASC"),
                GeoSortOrder::Desc => cmd.arg("DESC"),
            };
        }

        // Add count
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
            if options.count_any {
                cmd.arg("ANY");
            }
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;

        let results: Vec<GeoSearchResult> = result
            .into_iter()
            .filter_map(|v| {
                Self::parse_search_result(
                    v,
                    options.with_dist,
                    options.with_coord,
                    options.with_hash,
                )
            })
            .collect();

        Ok(results)
    }

    async fn geo_radius_by_member(
        &self,
        key: &str,
        member: &str,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        let mut conn = self.pool.get().await?;
        let mut cmd = redis::cmd("GEORADIUSBYMEMBER");
        cmd.arg(key).arg(member).arg(radius).arg(unit.as_str());

        // Add optional flags
        if options.with_coord {
            cmd.arg("WITHCOORD");
        }
        if options.with_dist {
            cmd.arg("WITHDIST");
        }
        if options.with_hash {
            cmd.arg("WITHHASH");
        }

        // Add sort order
        if let Some(sort) = &options.sort {
            match sort {
                GeoSortOrder::Asc => cmd.arg("ASC"),
                GeoSortOrder::Desc => cmd.arg("DESC"),
            };
        }

        // Add count
        if let Some(count) = options.count {
            cmd.arg("COUNT").arg(count);
            if options.count_any {
                cmd.arg("ANY");
            }
        }

        let result: Vec<Value> = cmd.query_async(&mut *conn).await?;

        let results: Vec<GeoSearchResult> = result
            .into_iter()
            .filter_map(|v| {
                Self::parse_search_result(
                    v,
                    options.with_dist,
                    options.with_coord,
                    options.with_hash,
                )
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_unit_as_str() {
        assert_eq!(GeoUnit::Meters.as_str(), "m");
        assert_eq!(GeoUnit::Kilometers.as_str(), "km");
        assert_eq!(GeoUnit::Miles.as_str(), "mi");
        assert_eq!(GeoUnit::Feet.as_str(), "ft");
    }

    #[test]
    fn test_geo_position() {
        let pos = GeoPosition::new(13.361389, 52.519444);
        assert_eq!(pos.longitude, 13.361389);
        assert_eq!(pos.latitude, 52.519444);
        assert!(pos.is_valid());
    }

    #[test]
    fn test_geo_member() {
        let member = GeoMember::new("Berlin", 13.361389, 52.519444);
        assert_eq!(member.member, "Berlin");
        assert_eq!(member.position.longitude, 13.361389);
        assert_eq!(member.position.latitude, 52.519444);
    }

    #[test]
    fn test_parse_search_result_simple() {
        let value = Value::BulkString(b"Berlin".to_vec());
        let result = RedisGeoRepository::parse_search_result(value, false, false, false);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.member, "Berlin");
        assert!(result.distance.is_none());
        assert!(result.position.is_none());
        assert!(result.geohash.is_none());
    }
}
