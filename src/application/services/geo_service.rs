//! Geo Service
//!
//! Business logic for Redis geospatial operations.

use std::sync::Arc;

use crate::domain::errors::CacheError;
use crate::domain::repositories::{
    GeoAddOptions, GeoAddResult, GeoMember, GeoPosition, GeoRepository, GeoSearchCenter,
    GeoSearchOptions, GeoSearchResult, GeoSearchShape, GeoSearchStoreResult, GeoUnit,
};
use crate::infrastructure::redis::connection::InstrumentedPool;
use crate::infrastructure::redis::repositories::RedisGeoRepository;

/// Service for geospatial operations
pub struct GeoService {
    repository: Arc<dyn GeoRepository>,
}

impl GeoService {
    /// Create a new GeoService with default Redis repository
    pub fn new(pool: Arc<InstrumentedPool>) -> Self {
        Self::new_with_repository(Arc::new(RedisGeoRepository::new(pool)))
    }

    /// Create a new GeoService with custom repository (useful for testing)
    pub fn new_with_repository(repository: Arc<dyn GeoRepository>) -> Self {
        Self { repository }
    }

    // ========== Core geo operations ==========

    /// GEOADD - Add one or more geospatial items (longitude, latitude, name) to a sorted set
    pub async fn geo_add(
        &self,
        key: &str,
        members: Vec<GeoMember>,
        options: GeoAddOptions,
    ) -> Result<GeoAddResult, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one member is required".to_string(),
            ));
        }
        // Validate coordinates
        for member in &members {
            Self::validate_position(&member.position)?;
        }
        // Validate options (NX and XX are mutually exclusive)
        if options.nx && options.xx {
            return Err(CacheError::InvalidInput(
                "NX and XX options are mutually exclusive".to_string(),
            ));
        }
        self.repository.geo_add(key, &members, options).await
    }

    /// GEOPOS - Get the positions (longitude, latitude) of specified members
    pub async fn geo_pos(
        &self,
        key: &str,
        members: Vec<String>,
    ) -> Result<Vec<Option<GeoPosition>>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one member is required".to_string(),
            ));
        }
        self.repository.geo_pos(key, &members).await
    }

    /// GEODIST - Get the distance between two members
    pub async fn geo_dist(
        &self,
        key: &str,
        member1: &str,
        member2: &str,
        unit: GeoUnit,
    ) -> Result<Option<f64>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if member1.is_empty() || member2.is_empty() {
            return Err(CacheError::InvalidInput(
                "Member names cannot be empty".to_string(),
            ));
        }
        self.repository.geo_dist(key, member1, member2, unit).await
    }

    /// GEOHASH - Get the geohash string representation of members
    pub async fn geo_hash(
        &self,
        key: &str,
        members: Vec<String>,
    ) -> Result<Vec<Option<String>>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if members.is_empty() {
            return Err(CacheError::InvalidInput(
                "At least one member is required".to_string(),
            ));
        }
        self.repository.geo_hash(key, &members).await
    }

    // ========== Search operations ==========

    /// GEOSEARCH - Search for members in a geographic area
    pub async fn geo_search(
        &self,
        key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        // Validate center if it's a position
        if let GeoSearchCenter::FromLonLat(pos) = &center {
            Self::validate_position(pos)?;
        }
        // Validate shape
        Self::validate_search_shape(&shape)?;
        self.repository.geo_search(key, center, shape, options).await
    }

    /// GEOSEARCHSTORE - Search and store results in a destination key
    pub async fn geo_search_store(
        &self,
        dest_key: &str,
        source_key: &str,
        center: GeoSearchCenter,
        shape: GeoSearchShape,
        options: GeoSearchOptions,
        store_dist: bool,
    ) -> Result<GeoSearchStoreResult, CacheError> {
        if dest_key.is_empty() {
            return Err(CacheError::InvalidInput(
                "Destination key cannot be empty".to_string(),
            ));
        }
        if source_key.is_empty() {
            return Err(CacheError::InvalidInput(
                "Source key cannot be empty".to_string(),
            ));
        }
        // Validate center if it's a position
        if let GeoSearchCenter::FromLonLat(pos) = &center {
            Self::validate_position(pos)?;
        }
        // Validate shape
        Self::validate_search_shape(&shape)?;
        self.repository
            .geo_search_store(dest_key, source_key, center, shape, options, store_dist)
            .await
    }

    // ========== Legacy operations (deprecated but still supported) ==========

    /// GEORADIUS - Query members within a radius from a point
    /// Note: Deprecated since Redis 6.2, use GEOSEARCH instead
    pub async fn geo_radius(
        &self,
        key: &str,
        position: GeoPosition,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        Self::validate_position(&position)?;
        if radius <= 0.0 {
            return Err(CacheError::InvalidInput(
                "Radius must be positive".to_string(),
            ));
        }
        self.repository
            .geo_radius(key, position, radius, unit, options)
            .await
    }

    /// GEORADIUSBYMEMBER - Query members within a radius from an existing member
    /// Note: Deprecated since Redis 6.2, use GEOSEARCH instead
    pub async fn geo_radius_by_member(
        &self,
        key: &str,
        member: &str,
        radius: f64,
        unit: GeoUnit,
        options: GeoSearchOptions,
    ) -> Result<Vec<GeoSearchResult>, CacheError> {
        if key.is_empty() {
            return Err(CacheError::InvalidInput("Key cannot be empty".to_string()));
        }
        if member.is_empty() {
            return Err(CacheError::InvalidInput(
                "Member name cannot be empty".to_string(),
            ));
        }
        if radius <= 0.0 {
            return Err(CacheError::InvalidInput(
                "Radius must be positive".to_string(),
            ));
        }
        self.repository
            .geo_radius_by_member(key, member, radius, unit, options)
            .await
    }

    // ========== Validation helpers ==========

    /// Validate a geographic position
    fn validate_position(pos: &GeoPosition) -> Result<(), CacheError> {
        if !pos.is_valid() {
            return Err(CacheError::InvalidInput(format!(
                "Invalid coordinates: longitude must be between -180 and 180, latitude must be between -85.05112878 and 85.05112878. Got: ({}, {})",
                pos.longitude, pos.latitude
            )));
        }
        Ok(())
    }

    /// Validate a search shape
    fn validate_search_shape(shape: &GeoSearchShape) -> Result<(), CacheError> {
        match shape {
            GeoSearchShape::ByRadius { radius, .. } => {
                if *radius <= 0.0 {
                    return Err(CacheError::InvalidInput(
                        "Radius must be positive".to_string(),
                    ));
                }
            }
            GeoSearchShape::ByBox { width, height, .. } => {
                if *width <= 0.0 || *height <= 0.0 {
                    return Err(CacheError::InvalidInput(
                        "Box width and height must be positive".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MockGeoRepository;

    #[tokio::test]
    async fn test_geo_add_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_add("", vec![], GeoAddOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_add_empty_members() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_add("locations", vec![], GeoAddOptions::default())
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_add_invalid_coordinates() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        // Invalid longitude (> 180)
        let err = service
            .geo_add(
                "locations",
                vec![GeoMember::new("test", 200.0, 45.0)],
                GeoAddOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_add_nx_xx_exclusive() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_add(
                "locations",
                vec![GeoMember::new("test", 13.361389, 52.519444)],
                GeoAddOptions {
                    nx: true,
                    xx: true,
                    ch: false,
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_add_success() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let result = service
            .geo_add(
                "locations",
                vec![
                    GeoMember::new("Berlin", 13.361389, 52.519444),
                    GeoMember::new("Paris", 2.349014, 48.864716),
                ],
                GeoAddOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(result.added, 2);
    }

    #[tokio::test]
    async fn test_geo_pos_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_pos("", vec!["member".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_pos_empty_members() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service.geo_pos("locations", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_hash_validation() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service.geo_hash("", vec!["member".to_string()]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service.geo_hash("locations", vec![]).await.unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_invalid_center() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search(
                "locations",
                GeoSearchCenter::FromLonLat(GeoPosition::new(200.0, 0.0)),
                GeoSearchShape::ByRadius {
                    radius: 10.0,
                    unit: GeoUnit::Meters,
                },
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_invalid_shape() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search(
                "locations",
                GeoSearchCenter::FromMember("Berlin".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 0.0,
                    unit: GeoUnit::Meters,
                },
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_store_validation() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search_store(
                "",
                "source",
                GeoSearchCenter::FromMember("Berlin".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 10.0,
                    unit: GeoUnit::Meters,
                },
                GeoSearchOptions::default(),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .geo_search_store(
                "dest",
                "",
                GeoSearchCenter::FromMember("Berlin".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 10.0,
                    unit: GeoUnit::Meters,
                },
                GeoSearchOptions::default(),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_store_invalid_center() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search_store(
                "dest",
                "source",
                GeoSearchCenter::FromLonLat(GeoPosition::new(200.0, 0.0)),
                GeoSearchShape::ByRadius {
                    radius: 10.0,
                    unit: GeoUnit::Meters,
                },
                GeoSearchOptions::default(),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_radius_invalid_position() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_radius(
                "locations",
                GeoPosition::new(-200.0, 0.0),
                10.0,
                GeoUnit::Meters,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_radius_by_member_validation() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_radius_by_member(
                "",
                "Berlin",
                10.0,
                GeoUnit::Meters,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .geo_radius_by_member(
                "locations",
                "",
                10.0,
                GeoUnit::Meters,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .geo_radius_by_member(
                "locations",
                "Berlin",
                0.0,
                GeoUnit::Meters,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_dist_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_dist("", "member1", "member2", GeoUnit::Meters)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_dist_empty_member() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_dist("locations", "", "member2", GeoUnit::Meters)
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_hash_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_hash("", vec!["member".to_string()])
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search(
                "",
                GeoSearchCenter::FromMember("test".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 100.0,
                    unit: GeoUnit::Kilometers,
                },
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_invalid_radius() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search(
                "locations",
                GeoSearchCenter::FromMember("test".to_string()),
                GeoSearchShape::ByRadius {
                    radius: -100.0,
                    unit: GeoUnit::Kilometers,
                },
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_invalid_box() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search(
                "locations",
                GeoSearchCenter::FromMember("test".to_string()),
                GeoSearchShape::ByBox {
                    width: 0.0,
                    height: 100.0,
                    unit: GeoUnit::Kilometers,
                },
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_search_store_empty_keys() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_search_store(
                "",
                "source",
                GeoSearchCenter::FromMember("test".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 100.0,
                    unit: GeoUnit::Kilometers,
                },
                GeoSearchOptions::default(),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));

        let err = service
            .geo_search_store(
                "dest",
                "",
                GeoSearchCenter::FromMember("test".to_string()),
                GeoSearchShape::ByRadius {
                    radius: 100.0,
                    unit: GeoUnit::Kilometers,
                },
                GeoSearchOptions::default(),
                false,
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_radius_empty_key() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_radius(
                "",
                GeoPosition::new(13.361389, 52.519444),
                100.0,
                GeoUnit::Kilometers,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_radius_invalid_radius() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_radius(
                "locations",
                GeoPosition::new(13.361389, 52.519444),
                -100.0,
                GeoUnit::Kilometers,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn test_geo_radius_by_member_empty_member() {
        let repo = Arc::new(MockGeoRepository::new());
        let service = GeoService::new_with_repository(repo);

        let err = service
            .geo_radius_by_member(
                "locations",
                "",
                100.0,
                GeoUnit::Kilometers,
                GeoSearchOptions::default(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, CacheError::InvalidInput(_)));
    }

    #[test]
    fn test_geo_service_new() {
        let pool = Arc::new(InstrumentedPool::new_for_tests());
        let _service = GeoService::new(pool);
    }
}
