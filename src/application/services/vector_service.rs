//! Vector Sets Service
//!
//! Service layer for Vector Sets operations.

use std::sync::Arc;
use tracing::instrument;

use crate::api::http::schemas::{
    VectorAddRequest, VectorAddResponse, VectorCardResponse, VectorDimResponse, VectorEmbRequest,
    VectorEmbResponse, VectorGetAttrResponse, VectorInfoResponse, VectorIsMemberRequest,
    VectorIsMemberResponse, VectorLinksResponse, VectorRandMemberRequest, VectorRandMemberResponse,
    VectorRangeRequest, VectorRangeResponse, VectorRemRequest, VectorRemResponse,
    VectorSetAttrRequest, VectorSetAttrResponse, VectorSimRequest, VectorSimResponse,
};
use crate::domain::errors::CacheError;
use crate::domain::repositories::VectorRepository;

/// Service for Vector Sets operations
#[derive(Clone)]
pub struct VectorService {
    repo: Arc<dyn VectorRepository>,
}

impl VectorService {
    /// Create a new VectorService
    pub fn new(repo: Arc<dyn VectorRepository>) -> Self {
        Self { repo }
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vadd(
        &self,
        key: &str,
        req: VectorAddRequest,
    ) -> Result<VectorAddResponse, CacheError> {
        let items: Vec<(String, Vec<f32>)> = req.items.into_iter().collect();
        let result = self.repo.vadd(key, items).await?;
        Ok(result.into())
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vrem(
        &self,
        key: &str,
        req: VectorRemRequest,
    ) -> Result<VectorRemResponse, CacheError> {
        let removed = self.repo.vrem(key, req.items).await?;
        Ok(VectorRemResponse {
            removed_count: removed,
        })
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vsim(
        &self,
        key: &str,
        req: VectorSimRequest,
    ) -> Result<VectorSimResponse, CacheError> {
        let result = self.repo.vsim(key, req.vector, req.k).await?;
        Ok(result.into())
    }

    #[instrument(skip(self), fields(key = %key))]
    pub async fn vcard(&self, key: &str) -> Result<VectorCardResponse, CacheError> {
        let count = self.repo.vcard(key).await?;
        Ok(VectorCardResponse { count })
    }

    #[instrument(skip(self), fields(key = %key))]
    pub async fn vdim(&self, key: &str) -> Result<VectorDimResponse, CacheError> {
        let dimension = self.repo.vdim(key).await?;
        Ok(VectorDimResponse { dimension })
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vemb(
        &self,
        key: &str,
        req: VectorEmbRequest,
    ) -> Result<VectorEmbResponse, CacheError> {
        let embeddings = self.repo.vemb(key, req.items).await?;
        Ok(VectorEmbResponse { embeddings })
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vismember(
        &self,
        key: &str,
        req: VectorIsMemberRequest,
    ) -> Result<VectorIsMemberResponse, CacheError> {
        let results = self.repo.vismember(key, req.items).await?;
        Ok(VectorIsMemberResponse { results })
    }

    #[instrument(skip(self), fields(key = %key, item = %item))]
    pub async fn vlinks(&self, key: &str, item: &str) -> Result<VectorLinksResponse, CacheError> {
        use crate::api::http::schemas::VectorLinksLayer;
        let layers_data = self.repo.vlinks(key, item).await?;
        let layers = layers_data
            .into_iter()
            .enumerate()
            .map(|(i, neighbors)| VectorLinksLayer {
                layer: i,
                neighbors,
            })
            .collect();
        Ok(VectorLinksResponse { layers })
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vrandmember(
        &self,
        key: &str,
        req: VectorRandMemberRequest,
    ) -> Result<VectorRandMemberResponse, CacheError> {
        let members = self.repo.vrandmember(key, req.count).await?;
        Ok(VectorRandMemberResponse { members })
    }

    #[instrument(skip(self, req), fields(key = %key))]
    pub async fn vrange(
        &self,
        key: &str,
        req: VectorRangeRequest,
    ) -> Result<VectorRangeResponse, CacheError> {
        let result = self
            .repo
            .vrange(key, &req.start, &req.end, req.count)
            .await?;
        Ok(result.into())
    }

    #[instrument(skip(self), fields(key = %key))]
    pub async fn vinfo(&self, key: &str) -> Result<VectorInfoResponse, CacheError> {
        let info = self.repo.vinfo(key).await?;
        Ok(info.into())
    }

    #[instrument(skip(self), fields(key = %key, item = %item))]
    pub async fn vgetattr(
        &self,
        key: &str,
        item: &str,
    ) -> Result<VectorGetAttrResponse, CacheError> {
        let attributes = self.repo.vgetattr(key, item).await?;
        Ok(VectorGetAttrResponse { attributes })
    }

    #[instrument(skip(self, req), fields(key = %key, item = %item))]
    pub async fn vsetattr(
        &self,
        key: &str,
        item: &str,
        req: VectorSetAttrRequest,
    ) -> Result<VectorSetAttrResponse, CacheError> {
        let success = self.repo.vsetattr(key, item, &req.attributes).await?;
        Ok(VectorSetAttrResponse { success })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::{
        VectorAddResult, VectorInfo, VectorItem, VectorRangeResult, VectorSimResult,
    };
    use crate::domain::errors::CacheError;
    use async_trait::async_trait;

    struct MockVectorRepo;

    #[async_trait]
    impl crate::domain::repositories::VectorRepository for MockVectorRepo {
        async fn vadd(
            &self,
            key: &str,
            items: Vec<(String, Vec<f32>)>,
        ) -> Result<VectorAddResult, CacheError> {
            Ok(VectorAddResult {
                key: key.to_string(),
                added_count: items.len() as u64,
            })
        }
        async fn vrem(&self, _key: &str, items: Vec<String>) -> Result<u64, CacheError> {
            Ok(items.len() as u64)
        }
        async fn vsim(
            &self,
            _key: &str,
            _vector: Vec<f32>,
            k: u64,
        ) -> Result<VectorSimResult, CacheError> {
            Ok(VectorSimResult {
                items: vec![
                    VectorItem {
                        id: "doc1".to_string(),
                        score: Some(0.95),
                        vector: None,
                        attributes: None,
                    },
                    VectorItem {
                        id: "doc2".to_string(),
                        score: Some(0.80),
                        vector: None,
                        attributes: None,
                    },
                ]
                .into_iter()
                .take(k as usize)
                .collect(),
            })
        }
        async fn vcard(&self, _key: &str) -> Result<u64, CacheError> {
            Ok(42)
        }
        async fn vdim(&self, _key: &str) -> Result<u64, CacheError> {
            Ok(128)
        }
        async fn vemb(
            &self,
            _key: &str,
            items: Vec<String>,
        ) -> Result<Vec<Option<Vec<f32>>>, CacheError> {
            Ok(items
                .into_iter()
                .map(|name| {
                    if name == "missing" {
                        None
                    } else {
                        Some(vec![1.0, 2.0, 3.0])
                    }
                })
                .collect())
        }
        async fn vismember(&self, _key: &str, items: Vec<String>) -> Result<Vec<bool>, CacheError> {
            Ok(items.into_iter().map(|name| name != "missing").collect())
        }
        async fn vlinks(&self, _key: &str, _item: &str) -> Result<Vec<Vec<String>>, CacheError> {
            Ok(vec![vec!["neighbor1".to_string(), "neighbor2".to_string()]])
        }
        async fn vrandmember(&self, _key: &str, count: i64) -> Result<Vec<String>, CacheError> {
            Ok((0..count.unsigned_abs())
                .map(|i| format!("member{}", i))
                .collect())
        }
        async fn vrange(
            &self,
            _key: &str,
            _start: &str,
            _end: &str,
            _count: Option<i64>,
        ) -> Result<VectorRangeResult, CacheError> {
            Ok(VectorRangeResult {
                items: vec![VectorItem {
                    id: "item0".to_string(),
                    score: None,
                    vector: None,
                    attributes: None,
                }],
            })
        }
        async fn vinfo(&self, _key: &str) -> Result<VectorInfo, CacheError> {
            Ok(VectorInfo {
                dimension: 128,
                distance_metric: "L2".to_string(),
                data_type: "FLOAT32".to_string(),
                count: 10,
            })
        }
        async fn vgetattr(&self, _key: &str, _item: &str) -> Result<Option<String>, CacheError> {
            Ok(Some(r#"{"category":"test"}"#.to_string()))
        }
        async fn vsetattr(
            &self,
            _key: &str,
            _item: &str,
            _attributes: &str,
        ) -> Result<bool, CacheError> {
            Ok(true)
        }
    }

    fn make_service() -> VectorService {
        VectorService::new(Arc::new(MockVectorRepo))
    }

    #[tokio::test]
    async fn test_vadd() {
        let svc = make_service();
        let mut items = std::collections::HashMap::new();
        items.insert("doc1".to_string(), vec![0.1, 0.2, 0.3]);
        items.insert("doc2".to_string(), vec![0.4, 0.5, 0.6]);
        let req = VectorAddRequest { items };
        let resp = svc.vadd("myvec", req).await.unwrap();
        assert_eq!(resp.added_count, 2);
        assert_eq!(resp.key, "myvec");
    }

    #[tokio::test]
    async fn test_vrem() {
        let svc = make_service();
        let req = VectorRemRequest {
            items: vec!["doc1".to_string(), "doc2".to_string()],
        };
        let resp = svc.vrem("myvec", req).await.unwrap();
        assert_eq!(resp.removed_count, 2);
    }

    #[tokio::test]
    async fn test_vsim() {
        let svc = make_service();
        let req = VectorSimRequest {
            vector: vec![0.1, 0.2, 0.3],
            k: 1,
        };
        let resp = svc.vsim("myvec", req).await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].id, "doc1");
        assert_eq!(resp.items[0].score, Some(0.95));
    }

    #[tokio::test]
    async fn test_vcard() {
        let svc = make_service();
        let resp = svc.vcard("myvec").await.unwrap();
        assert_eq!(resp.count, 42);
    }

    #[tokio::test]
    async fn test_vdim() {
        let svc = make_service();
        let resp = svc.vdim("myvec").await.unwrap();
        assert_eq!(resp.dimension, 128);
    }

    #[tokio::test]
    async fn test_vemb() {
        let svc = make_service();
        let req = VectorEmbRequest {
            items: vec!["doc1".to_string(), "missing".to_string()],
        };
        let resp = svc.vemb("myvec", req).await.unwrap();
        assert_eq!(resp.embeddings.len(), 2);
        assert!(resp.embeddings[0].is_some());
        assert!(resp.embeddings[1].is_none());
    }

    #[tokio::test]
    async fn test_vismember() {
        let svc = make_service();
        let req = VectorIsMemberRequest {
            items: vec!["doc1".to_string(), "missing".to_string()],
        };
        let resp = svc.vismember("myvec", req).await.unwrap();
        assert_eq!(resp.results, vec![true, false]);
    }

    #[tokio::test]
    async fn test_vlinks() {
        let svc = make_service();
        let resp = svc.vlinks("myvec", "doc1").await.unwrap();
        assert_eq!(resp.layers.len(), 1);
        assert_eq!(resp.layers[0].neighbors.len(), 2);
    }

    #[tokio::test]
    async fn test_vrandmember() {
        let svc = make_service();
        let req = VectorRandMemberRequest { count: 3 };
        let resp = svc.vrandmember("myvec", req).await.unwrap();
        assert_eq!(resp.members.len(), 3);
    }

    #[tokio::test]
    async fn test_vrange() {
        let svc = make_service();
        let req = VectorRangeRequest {
            start: "-".to_string(),
            end: "+".to_string(),
            count: None,
        };
        let resp = svc.vrange("myvec", req).await.unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].id, "item0");
    }

    #[tokio::test]
    async fn test_vinfo() {
        let svc = make_service();
        let resp = svc.vinfo("myvec").await.unwrap();
        assert_eq!(resp.dimension, 128);
        assert_eq!(resp.distance_metric, "L2");
        assert_eq!(resp.data_type, "FLOAT32");
        assert_eq!(resp.count, 10);
    }

    #[tokio::test]
    async fn test_vgetattr() {
        let svc = make_service();
        let resp = svc.vgetattr("myvec", "doc1").await.unwrap();
        assert!(resp.attributes.is_some());
        assert!(resp.attributes.unwrap().contains("category"));
    }

    #[tokio::test]
    async fn test_vsetattr() {
        let svc = make_service();
        let req = VectorSetAttrRequest {
            attributes: r#"{"tag":"new"}"#.to_string(),
        };
        let resp = svc.vsetattr("myvec", "doc1", req).await.unwrap();
        assert!(resp.success);
    }
}
