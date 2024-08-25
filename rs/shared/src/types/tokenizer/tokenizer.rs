use tiktoken_rs::{p50k_base, CoreBPE};

use crate::types::classifier::error::TokenizerError;

// const TOKEN_LIMIT: usize = 8192; // replace with your actual token limit

pub struct Tokenizer {
    bpe: CoreBPE,
}

impl Tokenizer {
    pub fn new() -> Result<Tokenizer, TokenizerError> {
        let bpe = p50k_base()?;
        Ok(Tokenizer { bpe })
    }

    pub fn calculate_token_length(&self, s: &str) -> usize {
        self.bpe.encode_with_special_tokens(s).len()
    }
}
