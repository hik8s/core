#[cfg(test)]
mod tests {
    use std::{iter::zip, path::Path, time::Instant};

    use async_openai::{error::OpenAIError, types::ChatCompletionRequestMessage};
    use bm25::{Language, SearchEngineBuilder};
    use chat_backend::chat::process::{process_user_message, RequestOptions};

    use data_vectorizer::vectorize::vectorizer::{vectorize_chunk, vectorize_class_batch};
    use rstest::rstest;
    use shared::{
        connections::{dbname::DbName, qdrant::EventQdrantMetadata},
        openai::OpenAIConnection,
        testdata::{UserTest, UserTestData},
        QdrantConnection,
    };
    use tokio::sync::mpsc;

    use shared::{
        connections::openai::messages::extract_message_content,
        constant::OPENAI_EMBEDDING_TOKEN_LIMIT, get_env_var, tracing::setup::setup_tracing,
        types::tokenizer::Tokenizer, utils::ratelimit::RateLimiter,
    };

    use crate::util::read_yaml_files;

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::PodKillOutOffMemory), DbName::Log)]
    async fn test_process_user_message(
        #[case] testdata: UserTestData,
        #[case] db: DbName,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        let tokenizer = Tokenizer::new().unwrap();
        let rate_limiter = RateLimiter::new(OPENAI_EMBEDDING_TOKEN_LIMIT);

        // Data ingestion
        let points = vectorize_class_batch(&[testdata.class.clone()], &tokenizer, &rate_limiter)
            .await
            .unwrap()
            .0;
        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();
        qdrant
            .upsert_points(points, &db, &customer_id)
            .await
            .unwrap();

        // Prompt processing
        let request_option = RequestOptions::new(&testdata.prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        tracing::debug!("Messages: {:#?}", messages);
        tracing::debug!("Answer: {}", answer);
        assert!(!answer.is_empty());
        for (message, expected_message) in zip(&messages, &testdata.messages) {
            assert!(std::mem::discriminant(message) == std::mem::discriminant(expected_message));
        }

        // log-retrieval evaluation
        let tool_content = extract_message_content(&messages[3]).unwrap_or_default();
        assert!(!answer.is_empty());

        // LLM check
        let start = Instant::now();
        let openai = OpenAIConnection::new();
        let match_str = "OOM killed exit code 137";

        let truncated_answer = if answer.len() > 300 {
            format!("{}...", &answer[..300])
        } else {
            answer.to_string()
        };

        let question = format!(
            "Does the following answer definitely conclude that the pod was {match_str}? {truncated_answer}"
        );

        let evaluation = openai.ask_atomic_question(&question).await.unwrap();
        tracing::info!("Atomic question took: {:?}", start.elapsed());

        // bm25 match
        let start = Instant::now();
        let search_engine =
            SearchEngineBuilder::<u32>::with_corpus(Language::English, UserTest::corpus()).build();
        let search_results = search_engine.search(&answer, 3);

        for result in search_results {
            tracing::info!(
                "Document ID: {}, Score: {}",
                result.document.id,
                result.score
            );
            tracing::info!("Document Content: {}", result.document.contents);
            tracing::info!("---");
        }
        assert!(evaluation, "evaluation: {evaluation} question: {question}");
        assert!(tool_content.contains(&testdata.class.to_string()));
        tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }

    fn get_event_qdrant_metadata() -> EventQdrantMetadata {
        let testdata_dir = Path::new(".testdata_examples");
        let path = testdata_dir.join("event");
        let json = read_yaml_files(&path).unwrap().pop().unwrap();
        let data = serde_yaml::to_string(&json).unwrap();
        EventQdrantMetadata::new(
            "v1".to_owned(),
            "Pod".to_owned(),
            "1234".to_owned(),
            "hello-server-597dbdc58-bz6zv".to_owned(),
            "examples".to_owned(),
            "Stopping container hello-server".to_owned(),
            "Killing".to_owned(),
            "Normal".to_owned(),
            data,
        )
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::RetrieveEvent), DbName::Event)]
    async fn test_process_user_input_event(
        #[case] testdata: UserTestData,
        #[case] db: DbName,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();
        // Data ingestion

        let event = get_event_qdrant_metadata();
        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();
        vectorize_chunk(
            &mut vec![event.data.clone()],
            &mut vec![event],
            &qdrant,
            &customer_id,
            &db,
            0,
        )
        .await;

        // Prompt processing
        let request_option = RequestOptions::new(&testdata.prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        tracing::debug!("Messages: {:#?}", messages);
        tracing::debug!("Answer: {}", answer);
        assert!(!answer.is_empty());
        assert_eq!(messages.len(), 4);
        for (message, expected_message) in zip(&messages, &testdata.messages) {
            assert!(std::mem::discriminant(message) == std::mem::discriminant(expected_message));
        }

        // log-retrieval evaluation
        // let tool_content = extract_message_content(&messages[3]).unwrap_or_default();
        assert!(!answer.is_empty());

        // // LLM check
        // let start = Instant::now();
        // let openai = OpenAIConnection::new();
        // let match_str = "OOM killed exit code 137";
        // let question = format!(
        //     "Does the following answer definitely conclude that the pod was {match_str}? {answer}"
        // );
        // let evaluation = openai.ask_atomic_question(&question).await.unwrap();
        // tracing::info!("Atomic question took: {:?}", start.elapsed());

        // // bm25 match
        // let start = Instant::now();
        // let search_engine =
        //     SearchEngineBuilder::<u32>::with_corpus(Language::English, UserTest::corpus()).build();
        // let search_results = search_engine.search(&answer, 3);

        // for result in search_results {
        //     tracing::info!(
        //         "Document ID: {}, Score: {}",
        //         result.document.id,
        //         result.score
        //     );
        //     tracing::info!("Document Content: {}", result.document.contents);
        //     tracing::info!("---");
        // }
        // assert!(evaluation, "evaluation: {evaluation} question: {question}");
        // assert!(tool_content.contains(&format!("{}", &testdata.class.to_string())));
        // tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::RetrieveResourceStatus))]
    async fn test_process_user_input_resource_status(
        #[case] testdata: UserTestData,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();

        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();

        // Prompt processing
        let request_option = RequestOptions::new(&testdata.prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        tracing::debug!("Messages: {:#?}", messages);
        tracing::debug!("Answer: {}", answer);
        assert!(!answer.is_empty());
        // assert_eq!(messages.len(), 4);
        for (message, expected_message) in zip(&messages, &testdata.messages) {
            assert!(std::mem::discriminant(message) == std::mem::discriminant(expected_message));
        }

        // log-retrieval evaluation
        // let tool_content = extract_message_content(&messages[3]).unwrap_or_default();
        assert!(!answer.is_empty());

        // // LLM check
        // let start = Instant::now();
        // let openai = OpenAIConnection::new();
        // let match_str = "OOM killed exit code 137";
        // let question = format!(
        //     "Does the following answer definitely conclude that the pod was {match_str}? {answer}"
        // );
        // let evaluation = openai.ask_atomic_question(&question).await.unwrap();
        // tracing::info!("Atomic question took: {:?}", start.elapsed());

        // // bm25 match
        // let start = Instant::now();
        // let search_engine =
        //     SearchEngineBuilder::<u32>::with_corpus(Language::English, UserTest::corpus()).build();
        // let search_results = search_engine.search(&answer, 3);

        // for result in search_results {
        //     tracing::info!(
        //         "Document ID: {}, Score: {}",
        //         result.document.id,
        //         result.score
        //     );
        //     tracing::info!("Document Content: {}", result.document.contents);
        //     tracing::info!("---");
        // }
        // assert!(evaluation, "evaluation: {evaluation} question: {question}");
        // assert!(tool_content.contains(&format!("{}", &testdata.class.to_string())));
        // tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }

    #[tokio::test]
    #[rstest]
    #[case(UserTestData::new(UserTest::RetrieveCustomResourceStatus))]
    async fn test_process_user_input_customresource_status(
        #[case] testdata: UserTestData,
    ) -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();

        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();

        // Prompt processing
        let request_option = RequestOptions::new(&testdata.prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();
        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        tracing::debug!("Messages: {:#?}", messages);
        tracing::debug!("Answer: {}", answer);
        assert!(!answer.is_empty());
        assert_eq!(messages.len(), 4);
        for (message, expected_message) in zip(&messages, &testdata.messages) {
            assert!(std::mem::discriminant(message) == std::mem::discriminant(expected_message));
        }

        // log-retrieval evaluation
        // let tool_content = extract_message_content(&messages[3]).unwrap_or_default();
        assert!(!answer.is_empty());

        // // LLM check
        // let start = Instant::now();
        // let openai = OpenAIConnection::new();
        // let match_str = "OOM killed exit code 137";
        // let question = format!(
        //     "Does the following answer definitely conclude that the pod was {match_str}? {answer}"
        // );
        // let evaluation = openai.ask_atomic_question(&question).await.unwrap();
        // tracing::info!("Atomic question took: {:?}", start.elapsed());

        // // bm25 match
        // let start = Instant::now();
        // let search_engine =
        //     SearchEngineBuilder::<u32>::with_corpus(Language::English, UserTest::corpus()).build();
        // let search_results = search_engine.search(&answer, 3);

        // for result in search_results {
        //     tracing::info!(
        //         "Document ID: {}, Score: {}",
        //         result.document.id,
        //         result.score
        //     );
        //     tracing::info!("Document Content: {}", result.document.contents);
        //     tracing::info!("---");
        // }
        // assert!(evaluation, "evaluation: {evaluation} question: {question}");
        // assert!(tool_content.contains(&format!("{}", &testdata.class.to_string())));
        // tracing::info!("Search and evaluation took: {:?}", start.elapsed());
        Ok(())
    }

    #[tokio::test]
    async fn prompt_playground() -> Result<(), OpenAIError> {
        setup_tracing(false);
        let qdrant = QdrantConnection::new().await.unwrap();

        // let prompt = "Can u create a deployment for my application?";
        let prompt =
        // "I wanna create a deployment for my application anything_else in namespace test1-namespace. The image name is also my_image1. What are my options for databases?";
        "I wanna create a deployment for my application anything_else in namespace test1-namespace. The image name is also my_image1.";
        // "I wanna create a deployment for my application anything_else in namespace test1-namespace. The image name is also my_image1. Create the deployment with postgres and kafka";
        let customer_id = get_env_var("AUTH0_CLIENT_ID_LOCAL").unwrap();

        // Prompt processing
        let request_option = RequestOptions::new(prompt, &customer_id);
        let mut messages: Vec<ChatCompletionRequestMessage> = request_option.clone().into();

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();
        process_user_message(&qdrant, &mut messages, &tx, request_option)
            .await
            .unwrap();

        // Answer evaluation
        let mut answer = String::new();
        rx.close();
        while let Some(message_delta) = rx.recv().await {
            answer.push_str(&message_delta);
        }
        tracing::debug!("\nMESSAGE\n: {:#?}", messages);
        tracing::debug!("\nANSWER\n: {}", answer);
        Ok(())
    }
}
