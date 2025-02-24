use std::sync::Arc;

use qdrant_client::{
    qdrant::{
        Condition, CreateCollectionBuilder, Distance, Filter, PointStruct, PointsOperationResponse,
        QueryPointsBuilder, ScoredPoint, SearchPointsBuilder, SetPayloadPointsBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
    Payload, Qdrant, QdrantError,
};
use rocket::{request::FromRequest, State};
use tonic::Code;
use tracing::info;

use crate::{
    constant::{EMBEDDING_SIZE, EMBEDDING_USIZE},
    QdrantConnectionError,
};

use super::config::QdrantConfig;

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

    pub async fn create_collection(&self, db: &str) -> Result<(), QdrantConnectionError> {
        let collections = self.client.list_collections().await?;
        for collection in collections.collections {
            if collection.name == db {
                return Ok(());
            }
        }

        match self
            .client
            .create_collection(
                CreateCollectionBuilder::new(db)
                    .vectors_config(VectorParamsBuilder::new(EMBEDDING_SIZE, Distance::Cosine)),
            )
            .await
        {
            Ok(_) => {
                info!("Collection {} created", db);
                Ok(())
            }
            Err(QdrantError::ResponseError { status }) if status.code() == Code::AlreadyExists => {
                Ok(())
            }
            Err(e) => Err(QdrantConnectionError::CreateCollection(e)),
        }
    }

    pub async fn upsert_points(
        &self,
        qdrant_point: Vec<PointStruct>,
        key: &str,
    ) -> Result<PointsOperationResponse, QdrantConnectionError> {
        self.create_collection(key).await?;
        let request = UpsertPointsBuilder::new(key, qdrant_point).wait(false);
        self.client
            .upsert_points(request)
            .await
            .map_err(QdrantConnectionError::UpsertPoints)
    }
    pub async fn set_payload(
        &self,
        db: &str,
        filter: Filter,
        new_payload: Payload,
    ) -> Result<PointsOperationResponse, QdrantError> {
        self.client
            .set_payload(
                SetPayloadPointsBuilder::new(db, new_payload)
                    .points_selector(filter)
                    .wait(true),
            )
            .await
    }
    pub async fn search_points(
        &self,
        db: &str,
        array: [f32; EMBEDDING_USIZE],
        mut filter: Filter,
        limit: u64,
    ) -> Result<Vec<ScoredPoint>, QdrantConnectionError> {
        self.create_collection(db).await?;
        filter.must_not.push(Condition::matches("deleted", true));
        let request = SearchPointsBuilder::new(db, array.to_vec(), limit)
            .filter(filter)
            .with_payload(true);
        let response = self.client.search_points(request).await?;
        Ok(response.result)
    }
    pub async fn query_points(
        &self,
        db: &str,
        filter: Option<Filter>,
        limit: u64,
        with_payload: bool,
    ) -> Result<Vec<ScoredPoint>, QdrantConnectionError> {
        let mut request = QueryPointsBuilder::new(db)
            .limit(limit)
            .with_payload(with_payload);

        if let Some(filter) = filter {
            request = request.filter(filter);
        }
        let response = self.client.query(request).await?;
        Ok(response.result)
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

pub fn create_filter_with_data_type(
    namespace: Option<&String>,
    application: Option<&String>,
    data_type: &str,
) -> Filter {
    let mut conditions = Vec::new();
    conditions.push(Condition::matches("data_type", data_type.to_owned()));
    if let Some(val) = namespace {
        conditions.push(Condition::matches("namespace", val.to_owned()));
    }
    if let Some(val) = application {
        conditions.push(Condition::matches("key", val.to_owned()));
    }

    Filter::must(conditions)
}

pub fn match_any(key: &str, values: &[String]) -> Filter {
    let conditions: Vec<Condition> = values
        .iter()
        .map(|v| Condition::matches(key, v.to_owned()))
        .collect();
    Filter::should(conditions)
}

pub async fn update_deleted_resources(
    qdrant: &QdrantConnection,
    db: &str,
    uids: &[String],
) -> Result<(), QdrantConnectionError> {
    // Skip if no points to update
    if uids.is_empty() {
        return Ok(());
    }

    // Create filter for matching UIDs
    let filter = match_any("resource_uid", uids);

    // Create payload with deleted flag
    let mut payload = Payload::new();
    payload.insert("deleted", true);

    // Update points in batch
    qdrant
        .set_payload(db, filter, payload)
        .await
        .map_err(QdrantConnectionError::SetPayload)?;

    Ok(())
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

pub fn string_filter(key: &str, value: &str) -> Filter {
    Filter::must([Condition::matches(key, value.to_owned())])
}
pub fn bool_filter(key: &str) -> Filter {
    Filter::must([Condition::matches(key, true)])
}

pub fn string_condition(key: &str, value: &str) -> Condition {
    Condition::matches(key, value.to_owned())
}

pub fn parse_qdrant_value(
    data: &qdrant_client::qdrant::Value,
) -> (serde_yaml::Value, serde_json::Value) {
    let content = data.as_str().unwrap();

    // Parse YAML first
    let yaml: serde_yaml::Value = serde_yaml::from_str(content).unwrap();

    // Convert to JSON without re-parsing
    let json = serde_json::to_value(&yaml).unwrap();

    (yaml, json)
}
