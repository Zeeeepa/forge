//! Index service that manages the vector store

use std::env;

use qdrant_client::qdrant::vectors_config::Config as VectorConfig;
use qdrant_client::qdrant::with_payload_selector::SelectorOptions;
use qdrant_client::qdrant::{
    CreateCollectionBuilder, DeletePointsBuilder, Distance, PointStruct, SearchPointsBuilder,
    UpsertPointsBuilder, VectorParams, VectorsConfig,
};
use qdrant_client::{Payload, Qdrant};
use tracing::{debug, error, info, warn};

use crate::proto::Chunk;
use crate::{ForgeIndexerError, Result};

type Embedding = Vec<f32>;

pub struct IndexService {
    client: Qdrant,
    collection_name: String,
    vector_dimension: usize,
}

impl IndexService {
    pub async fn new(vector_dimension: usize) -> Result<Self> {
        info!("🔧 Initializing IndexService...");

        // Get Qdrant configuration from environment variables
        let qdrant_url =
            env::var("QDRANT_URL").unwrap_or_else(|_| "http://localhost:6334".to_string());
        // Resolve namespace prefix for branch/user isolation
        let namespace_prefix =
            env::var("QDRANT_NAMESPACE_PREFIX").unwrap_or_else(|_| "".to_string());
        // Base collection name
        let base_collection =
            env::var("QDRANT_COLLECTION").unwrap_or_else(|_| "forge-indexer".to_string());
        // Compose final collection name with namespace
        let collection_name = if namespace_prefix.is_empty() {
            base_collection.clone()
        } else {
            format!("{namespace_prefix}-{base_collection}")
        };

        info!("🔗 Connecting to Qdrant at: {}", qdrant_url);
        info!("📦 Using collection: {}", collection_name);
        info!("📏 Vector dimension: {}", vector_dimension);

        // Create Qdrant client
        let client = match Qdrant::from_url(&qdrant_url).build() {
            Ok(client) => {
                info!("✅ Successfully connected to Qdrant");
                client
            }
            Err(e) => {
                error!("❌ Failed to connect to Qdrant: {}", e);
                return Err(ForgeIndexerError::vector_db_error_with_source(
                    "Failed to connect to Qdrant",
                    e,
                ));
            }
        };

        // Create collection if it doesn't exist or recreate if dimensions changed
        info!("🔍 Checking if collection exists...");
        let collections = client.list_collections().await?;
        let collection_exists = collections
            .collections
            .iter()
            .any(|c| c.name == collection_name);

        if collection_exists {
            info!("📦 Collection exists, checking if dimensions match...");
            // Check if we need to recreate due to dimension mismatch
            match client.collection_info(&collection_name).await {
                Ok(info) => {
                    if let Some(config) = &info.result.and_then(|r| r.config) {
                        if let Some(vectors_config) = &config
                            .params
                            .as_ref()
                            .and_then(|p| p.vectors_config.as_ref())
                        {
                            if let Some(VectorConfig::Params(params)) = &vectors_config.config {
                                if params.size as usize != vector_dimension {
                                    warn!(
                                        "🔄 Vector dimension mismatch! Collection has {} but need {}. Recreating collection...",
                                        params.size, vector_dimension
                                    );
                                    Self::recreate_collection(
                                        &client,
                                        &collection_name,
                                        vector_dimension,
                                    )
                                    .await?;
                                } else {
                                    info!(
                                        "✅ Collection dimensions match, using existing collection"
                                    );
                                }
                            } else {
                                warn!(
                                    "⚠️  Could not determine collection dimensions, recreating to be safe..."
                                );
                                Self::recreate_collection(
                                    &client,
                                    &collection_name,
                                    vector_dimension,
                                )
                                .await?;
                            }
                        } else {
                            warn!("⚠️  Could not get vector config, recreating collection...");
                            Self::recreate_collection(&client, &collection_name, vector_dimension)
                                .await?;
                        }
                    } else {
                        warn!("⚠️  Could not get collection config, recreating collection...");
                        Self::recreate_collection(&client, &collection_name, vector_dimension)
                            .await?;
                    }
                }
                Err(e) => {
                    warn!(
                        "⚠️  Could not get collection info: {}. Recreating collection...",
                        e
                    );
                    Self::recreate_collection(&client, &collection_name, vector_dimension).await?;
                }
            }
        } else {
            info!("📝 Creating new collection: {}", collection_name);
            Self::create_collection(&client, &collection_name, vector_dimension).await?;
            info!("✅ Collection created successfully");
        }

        info!("✅ IndexService initialization complete");
        Ok(Self { client, collection_name, vector_dimension })
    }

    async fn create_collection(
        client: &Qdrant,
        collection_name: &str,
        vector_dimension: usize,
    ) -> Result<()> {
        client
            .create_collection(
                CreateCollectionBuilder::new(collection_name.to_string())
                    .vectors_config(VectorsConfig {
                        config: Some(VectorConfig::Params(VectorParams {
                            size: vector_dimension as u64,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    })
                    .build(),
            )
            .await?;
        Ok(())
    }

    async fn recreate_collection(
        client: &Qdrant,
        collection_name: &str,
        vector_dimension: usize,
    ) -> Result<()> {
        info!("🗑️  Deleting existing collection: {}", collection_name);
        if let Err(e) = client.delete_collection(collection_name).await {
            warn!("Failed to delete collection (may not exist): {}", e);
        }

        info!(
            "📝 Creating new collection with {} dimensions",
            vector_dimension
        );
        Self::create_collection(client, collection_name, vector_dimension).await?;
        info!("✅ Collection recreated successfully");
        Ok(())
    }

    /// Reset the collection by deleting and recreating it
    pub async fn reset_collection(&self) -> Result<()> {
        info!("🔄 Resetting collection: {}", self.collection_name);
        Self::recreate_collection(&self.client, &self.collection_name, self.vector_dimension).await
    }

    pub async fn add_embedding(
        &mut self,
        id: &str,
        embedding: Embedding,
        payload: Payload,
    ) -> Result<()> {
        debug!("🗂️  Adding embedding for chunk: {}", id);

        // Use a valid UUID for the point ID
        let point_id = uuid::Uuid::new_v4().to_string();
        let points = vec![PointStruct::new(point_id.clone(), embedding, payload)];

        match self
            .client
            .upsert_points(UpsertPointsBuilder::new(
                self.collection_name.clone(),
                points,
            ))
            .await
        {
            Ok(_) => {
                debug!(
                    "✅ Successfully added embedding for chunk: {} with point ID: {}",
                    id, point_id
                );
                Ok(())
            }
            Err(e) => {
                error!("❌ Failed to add embedding for chunk {}: {}", id, e);
                Err(ForgeIndexerError::vector_db_error_with_source(
                    "Failed to add embedding to Qdrant",
                    e,
                ))
            }
        }
    }

    pub async fn search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f32)>> {
        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(self.collection_name.clone(), query.to_vec(), k as u64)
                    .with_payload(SelectorOptions::Enable(true)),
            )
            .await
            .map_err(|e| {
                ForgeIndexerError::vector_db_error_with_source("Failed to search in Qdrant", e)
            })?;

        let mut results = Vec::new();
        for result in response.result {
            if let Some(id) = &result.id {
                results.push((format!("{id:?}"), result.score));
            }
        }

        Ok(results)
    }

    pub async fn delete(&mut self, id: &str) -> Result<()> {
        self.client
            .delete_points(
                DeletePointsBuilder::new(self.collection_name.clone()).points(vec![id.to_string()]),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete point from Qdrant: {}", e))?;

        Ok(())
    }

    /// Search for similar chunks with optional filtering
    pub async fn search_similar(
        &self,
        query_embedding: &[f32],
        k: usize,
        repo_filter: Option<&str>,
        branch_filter: Option<&str>,
    ) -> Result<Vec<(Chunk, f32)>> {
        info!(
            "Starting vector search with k={}, repo_filter={:?}, branch_filter={:?}",
            k, repo_filter, branch_filter
        );

        let response = self
            .client
            .search_points(
                SearchPointsBuilder::new(
                    self.collection_name.clone(),
                    query_embedding.to_vec(),
                    k as u64,
                )
                .with_payload(SelectorOptions::Enable(true)),
            )
            .await
            .map_err(|e| {
                error!("Failed to search in Qdrant: {}", e);
                anyhow::anyhow!("Failed to search in Qdrant: {}", e)
            })?;

        info!("Qdrant returned {} raw results", response.result.len());

        let mut results = Vec::new();
        for (idx, result) in response.result.iter().enumerate() {
            let payload = result.payload.clone();

            info!("Processing result {}: score={:.4}", idx, result.score);

            // Apply filters first before converting to chunk to avoid borrow issues
            let matches_repo = repo_filter.is_none_or(|repo| {
                // More flexible repo matching - check if the repo name appears anywhere in the path
                // or if the path contains the repo as a substring
                if let Some(path_value) = payload.get("path") {
                    if let Some(qdrant_client::qdrant::value::Kind::StringValue(path)) =
                        &path_value.kind
                    {
                        // Check multiple matching strategies:
                        // 1. Exact path component match (original logic)
                        let path_parts: Vec<&str> = path.split('/').collect();
                        let exact_match = path_parts.contains(&repo);

                        // 2. Substring match (more flexible)
                        let substring_match = path.contains(repo);

                        // 3. Special case: if repo is "." or empty, match all
                        let wildcard_match = repo.is_empty() || repo == "." || repo == "*";

                        let matches = exact_match || substring_match || wildcard_match;
                        info!(
                            "Repo filter check: path='{}', repo='{}', exact={}, substring={}, wildcard={}, final_match={}",
                            path, repo, exact_match, substring_match, wildcard_match, matches
                        );
                        matches
                    } else {
                        warn!("Path value is not a string in payload");
                        false
                    }
                } else {
                    warn!("No path field found in payload");
                    false
                }
            });

            let matches_branch = branch_filter.is_none_or(|branch| {
                // Branch filtering requires metadata in payload
                let matches = payload
                    .get("branch")
                    .and_then(|v| v.kind.as_ref())
                    .and_then(|kind| match kind {
                        qdrant_client::qdrant::value::Kind::StringValue(s) => Some(s),
                        _ => None,
                    })
                    .is_some_and(|b| b == branch);
                info!(
                    "Branch filter check: branch_filter='{}', matches={}",
                    branch, matches
                );
                matches
            });

            // Only convert to chunk if filters pass
            if matches_repo && matches_branch {
                match payload_to_chunk(payload, result.score) {
                    Ok(chunk) => {
                        info!(
                            "Added chunk: path='{}', symbol={:?}",
                            chunk.path, chunk.symbol
                        );
                        results.push((chunk, result.score));
                    }
                    Err(e) => {
                        warn!("Failed to convert payload to chunk: {}", e);
                    }
                }
            } else {
                info!(
                    "Result {} filtered out: repo_match={}, branch_match={}",
                    idx, matches_repo, matches_branch
                );
            }
        }

        info!("Returning {} filtered results", results.len());
        Ok(results)
    }
}

/// Convert Qdrant payload back to Chunk
fn payload_to_chunk(
    payload: std::collections::HashMap<String, qdrant_client::qdrant::Value>,
    _score: f32,
) -> Result<Chunk> {
    let get_string = |key: &str| -> Result<String> {
        payload
            .get(key)
            .and_then(|v| match v.kind {
                Some(qdrant_client::qdrant::value::Kind::StringValue(ref s)) => Some(s.clone()),
                _ => None,
            })
            .ok_or_else(|| ForgeIndexerError::validation_error(key, "Missing or invalid field"))
    };

    let get_optional_string = |key: &str| -> Option<String> {
        payload.get(key).and_then(|v| match v.kind {
            Some(qdrant_client::qdrant::value::Kind::StringValue(ref s)) => Some(s.clone()),
            _ => None,
        })
    };

    let get_usize = |key: &str| -> Result<usize> {
        payload
            .get(key)
            .and_then(|v| match v.kind {
                Some(qdrant_client::qdrant::value::Kind::IntegerValue(n)) => Some(n as usize),
                _ => None,
            })
            .ok_or_else(|| ForgeIndexerError::validation_error(key, "Missing or invalid field"))
    };

    Ok(Chunk {
        id: uuid::Uuid::new_v4().to_string(), // Generate new ID for search results
        path: get_string("path")?,
        lang: get_string("lang")?,
        symbol: get_optional_string("symbol"),
        rev: get_string("rev")?,
        size: get_usize("size")?,
        code: get_string("code")?,
        summary: get_optional_string("summary"),
        embedding: None, // Don't include embedding in search results
    })
}
