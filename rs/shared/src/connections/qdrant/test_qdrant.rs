#[cfg(test)]
mod tests {
    use crate::{
        connections::{
            dbname::DbName,
            qdrant::{
                connect::{match_any, update_deleted_resources, QdrantConnection},
                error::QdrantConnectionError,
            },
        },
        constant::EMBEDDING_SIZE,
        tracing::setup::setup_tracing,
    };

    use qdrant_client::{
        qdrant::{PointStruct, Value},
        Payload,
    };
    use uuid7::uuid4;

    fn create_test_point(resource_uid: &str) -> PointStruct {
        let mut payload = Payload::new();
        payload.insert("name", "test_name");
        payload.insert("resource_uid", resource_uid);
        payload.insert("version", "1.0");
        payload.insert("deleted", false);

        PointStruct::new(
            uuid4().to_string(),
            vec![0.1; EMBEDDING_SIZE as usize],
            payload,
        )
    }

    #[tokio::test]
    async fn test_update_query_search_deleted() -> Result<(), QdrantConnectionError> {
        // Setup
        setup_tracing(true);
        let qdrant = QdrantConnection::new().await?;
        let customer_id = "test_customer";
        let db = DbName::Log;

        // Create initial point with multiple fields
        let uid1 = uuid4().to_string();
        let uid2 = uuid4().to_string();
        let uid3 = uuid4().to_string();

        let point1 = create_test_point(&uid1);
        let point2 = create_test_point(&uid2);
        let point3 = create_test_point(&uid3);

        // Insert point
        qdrant
            .upsert_points(vec![point1, point2, point3], &db, customer_id)
            .await?;

        // Update deleted field
        let mut resource_uids = vec![uid1, uid2];

        update_deleted_resources(&qdrant, customer_id, &db, &resource_uids).await?;

        // Verify update
        let db = DbName::Log;

        let filter_uid12 = match_any("resource_uid", &resource_uids);
        let points = qdrant
            .query_points(&db, customer_id, filter_uid12, 1000)
            .await?;
        assert_eq!(points.len(), 2);
        for point in points.iter() {
            let payload = &point.payload;
            assert_eq!(payload.get("deleted"), Some(&Value::from(true)));
            assert_eq!(payload.get("name"), Some(&Value::from("test_name")));
            assert_eq!(payload.get("version"), Some(&Value::from("1.0")));
        }

        let array = [0.1; EMBEDDING_SIZE as usize];
        resource_uids.push(uid3);
        let filter_uid123 = match_any("resource_uid", &resource_uids);

        let points = qdrant
            .search_points(&db, customer_id, array, filter_uid123, 1000)
            .await?;
        for point in points.iter() {
            let payload = &point.payload;
            assert_eq!(payload.get("deleted"), Some(&Value::from(false)));
            assert_eq!(payload.get("name"), Some(&Value::from("test_name")));
            assert_eq!(payload.get("version"), Some(&Value::from("1.0")));
        }
        assert_eq!(points.len(), 1);

        Ok(())
    }
}
