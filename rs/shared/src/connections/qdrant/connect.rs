use std::sync::Arc;

use qdrant_client::{
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, PointsOperationResponse,
        QueryPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    },
    Qdrant,
};
use rocket::{request::FromRequest, State};
use tracing::info;

use crate::{
    constant::{EMBEDDING_SIZE, EMBEDDING_USIZE},
    types::class::vectorized::{from_scored_point, VectorizedClass},
};

use super::{config::QdrantConfig, error::QdrantConnectionError};

#[derive(Clone)]
pub struct QdrantConnection {
    pub client: Arc<Qdrant>,
    pub config: QdrantConfig,
}

impl QdrantConnection {
    pub async fn new() -> Result<Self, QdrantConnectionError> {
        let config = QdrantConfig::new()?;

        // Qdrant client
        let client = Arc::new(Qdrant::from_url(&config.get_qdrant_uri()).build().unwrap());

        let connection = QdrantConnection { client, config };

        // Create collection
        Ok(connection)
    }

    pub async fn create_collection(&self, db_name: &str) -> Result<(), QdrantConnectionError> {
        let collections = self.client.list_collections().await?;
        for collection in collections.collections {
            if collection.name == db_name {
                return Ok(());
            }
        }

        self.client
            .create_collection(
                CreateCollectionBuilder::new(db_name.to_owned())
                    .vectors_config(VectorParamsBuilder::new(EMBEDDING_SIZE, Distance::Cosine)),
            )
            .await?;
        info!("Collection {} created", db_name);
        Ok(())
    }
    pub async fn upsert_points(
        &self,
        qdrant_point: Vec<PointStruct>,
        db_name: &str,
    ) -> Result<PointsOperationResponse, QdrantConnectionError> {
        let request = UpsertPointsBuilder::new(db_name.to_owned(), qdrant_point).wait(false);
        let response = self.client.upsert_points(request).await?;
        Ok(response)
    }
    pub async fn search_classes(
        &self,
        db_name: &str,
        array: [f32; EMBEDDING_USIZE],
        filter: Filter,
        limit: u64,
    ) -> Result<Vec<VectorizedClass>, QdrantConnectionError> {
        let request = SearchPointsBuilder::new(db_name.to_owned(), array.to_vec(), limit)
            .filter(filter)
            .with_payload(true);
        let response = self.client.search_points(request).await?;
        let vectorized_classes = from_scored_point(response.result)?;
        Ok(vectorized_classes)
    }
    pub async fn search_key(
        &self,
        db_name: &str,
        key: &str,
        limit: u64,
    ) -> Result<Vec<VectorizedClass>, QdrantConnectionError> {
        let filter = Filter::must([Condition::matches("key", key.to_string())]);
        let request = QueryPointsBuilder::new(db_name.to_owned())
            .filter(filter)
            .limit(limit)
            .with_payload(true);
        let response = self.client.query(request).await?;
        let vectorized_classes = from_scored_point(response.result)?;
        Ok(vectorized_classes)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for QdrantConnection {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let connection = request.guard::<&State<QdrantConnection>>().await.unwrap();
        rocket::request::Outcome::Success(connection.inner().clone())
    }
}

#[rocket::async_trait]
impl rocket::fairing::Fairing for QdrantConnection {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Qdrant connection",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
        Ok(rocket.manage(self.clone()))
    }
}

pub fn namespace(namespace: &str) -> Filter {
    Filter::must(vec![Condition::matches(
        "namespace".to_string(),
        namespace.to_string(),
    )])
}

pub fn not_namespace(namespace: &str) -> Filter {
    Filter::must_not(vec![Condition::matches(
        "namespace".to_string(),
        namespace.to_string(),
    )])
}
