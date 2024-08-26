const EMBEDDING_MODEL: &str = "text-embedding-3-large";

use async_openai::{error::OpenAIError, types::CreateEmbeddingRequestArgs, Client};
use shared::types::classification::vectorized::EMBEDDING_USIZE;

pub async fn request_embedding(text: String) -> Result<[f32; 3072], OpenAIError> {
    let client = Client::new();

    let request = CreateEmbeddingRequestArgs::default()
        .model(EMBEDDING_MODEL)
        .input(text)
        .build()?;

    let response = client.embeddings().create(request).await?;

    let embedding = response.data[0].embedding.as_slice();
    let array: [f32; EMBEDDING_USIZE] = embedding.try_into().unwrap();

    Ok(array)
}
