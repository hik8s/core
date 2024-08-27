use crate::constant::{EMBEDDING_USIZE, OPENAI_EMBEDDING_MODEL};
use async_openai::{error::OpenAIError, types::CreateEmbeddingRequestArgs, Client};

pub async fn request_embedding(text: String) -> Result<[f32; 3072], OpenAIError> {
    let client = Client::new();

    let request = CreateEmbeddingRequestArgs::default()
        .model(OPENAI_EMBEDDING_MODEL)
        .input(text)
        .build()?;

    let response = client.embeddings().create(request).await?;

    let embedding = response.data[0].embedding.as_slice();
    let array: [f32; EMBEDDING_USIZE] = embedding.try_into().unwrap();

    Ok(array)
}
