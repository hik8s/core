use tiktoken_rs::{p50k_base, CoreBPE};

use crate::types::classifier::error::TokenizerError;

const TOKEN_LIMIT: usize = 8192;

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

    pub fn clip_tail(&self, s: String) -> (String, usize) {
        let token_count = self.calculate_token_length(&s);

        if token_count > TOKEN_LIMIT {
            let new_length = (s.len() as f64 * 0.9) as usize;
            let new_s = s.chars().take(new_length).collect();
            self.clip_tail(new_s)
        } else {
            (s, token_count)
        }
    }
}
