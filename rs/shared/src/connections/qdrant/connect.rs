use std::sync::Arc;

use qdrant_client::{
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, PointsOperationResponse,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
    Qdrant,
};
use tracing::info;

use crate::types::classification::vectorized::EMBEDDING_SIZE;

use super::{config::QdrantConfig, error::QdrantConnectionError};

#[derive(Clone)]
pub struct QdrantConnection {
    pub client: Arc<Qdrant>,
    pub config: QdrantConfig,
}

impl QdrantConnection {
    pub async fn new(collection_name: String) -> Result<Self, QdrantConnectionError> {
        let config = QdrantConfig::new(collection_name)?;

        // Qdrant client
        let client = Arc::new(Qdrant::from_url(&config.get_qdrant_uri()).build().unwrap());

        let connection = QdrantConnection { client, config };

        // Create collection
        connection.create_collection().await?;
        Ok(connection)
    }

    async fn create_collection(&self) -> Result<(), QdrantConnectionError> {
        let collections = self.client.list_collections().await?;
        for collection in collections.collections {
            if collection.name == self.config.collection_name {
                info!(
                    "Collection '{}' already exists",
                    self.config.collection_name
                );
                return Ok(());
            }
        }

        self.client
            .create_collection(
                CreateCollectionBuilder::new(self.config.collection_name.to_owned())
                    .vectors_config(VectorParamsBuilder::new(EMBEDDING_SIZE, Distance::Cosine)),
            )
            .await?;
        Ok(())
    }
    pub async fn upsert_point(
        &self,
        qdrant_point: PointStruct,
    ) -> Result<PointsOperationResponse, QdrantConnectionError> {
        let response = self
            .client
            .upsert_points(
                UpsertPointsBuilder::new(
                    self.config.collection_name.to_owned(),
                    vec![qdrant_point],
                )
                .wait(false),
            )
            .await?;
        Ok(response)
    }
}
