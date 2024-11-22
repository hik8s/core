use std::sync::Arc;

use qdrant_client::{
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, PointsOperationResponse,
        QueryPointsBuilder, SearchPointsBuilder, UpsertPointsBuilder, VectorParamsBuilder,
    },
    Qdrant, QdrantError,
};
use rocket::{request::FromRequest, State};
use tonic::Code;
use tracing::info;

use crate::{
    connections::dbname::DbName,
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

    pub async fn create_collection(
        &self,
        db: &DbName,
        customer_id: &str,
    ) -> Result<(), QdrantConnectionError> {
        let collections = self.client.list_collections().await?;
        for collection in collections.collections {
            if collection.name == db.id(customer_id) {
                return Ok(());
            }
        }

        match self
            .client
            .create_collection(
                CreateCollectionBuilder::new(db.id(customer_id))
                    .vectors_config(VectorParamsBuilder::new(EMBEDDING_SIZE, Distance::Cosine)),
            )
            .await
        {
            Ok(_) => (),
            Err(QdrantError::ResponseError { status }) if status.code() == Code::AlreadyExists => {
                return Ok(())
            }
            Err(e) => return Err(e.into()),
        };
        info!("Collection {} created", db);
        Ok(())
    }
    pub async fn upsert_points(
        &self,
        qdrant_point: Vec<PointStruct>,
        db: &DbName,
        customer_id: &str,
    ) -> Result<PointsOperationResponse, QdrantConnectionError> {
        self.create_collection(&db, customer_id).await?;
        let request = UpsertPointsBuilder::new(db.id(customer_id), qdrant_point).wait(false);
        let response = self.client.upsert_points(request).await?;
        Ok(response)
    }
    pub async fn search_classes(
        &self,
        db: &DbName,
        customer_id: &str,
        array: [f32; EMBEDDING_USIZE],
        filter: Filter,
        limit: u64,
    ) -> Result<Vec<VectorizedClass>, QdrantConnectionError> {
        let request = SearchPointsBuilder::new(db.id(customer_id), array.to_vec(), limit)
            .filter(filter)
            .with_payload(true);
        let response = self.client.search_points(request).await?;
        let vectorized_classes = from_scored_point(response.result)?;
        Ok(vectorized_classes)
    }
    pub async fn search_key(
        &self,
        db: &DbName,
        customer_id: &str,
        key: &str,
        limit: u64,
    ) -> Result<Vec<VectorizedClass>, QdrantConnectionError> {
        let filter = Filter::must([Condition::matches("key", key.to_string())]);
        let request = QueryPointsBuilder::new(db.id(customer_id))
            .filter(filter)
            .limit(limit)
            .with_payload(true);
        let response = self.client.query(request).await?;
        let vectorized_classes = from_scored_point(response.result)?;
        Ok(vectorized_classes)
    }
}

pub fn create_filter(namespace: Option<&String>, application: Option<&String>) -> Filter {
    let mut conditions = Vec::new();
    if let Some(val) = namespace {
        conditions.push(Condition::matches("namespace", val.to_owned()));
    }
    if let Some(val) = application {
        conditions.push(Condition::matches("key", val.to_owned()));
    }

    Filter::must(conditions)
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
