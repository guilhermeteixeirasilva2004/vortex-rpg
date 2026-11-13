//! Codec JSON (fácil de depurar). Trocar por um formato binário
//! (bincode/postcard) quando o volume de chunks começar a pesar.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Falha ao codificar ou decodificar uma mensagem.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ProtocolError(#[from] serde_json::Error);

/// Serializa uma mensagem para texto.
pub fn encode<T: Serialize>(msg: &T) -> Result<String, ProtocolError> {
    Ok(serde_json::to_string(msg)?)
}

/// Lê uma mensagem de texto.
pub fn decode<'a, T: Deserialize<'a>>(text: &'a str) -> Result<T, ProtocolError> {
    Ok(serde_json::from_str(text)?)
}
