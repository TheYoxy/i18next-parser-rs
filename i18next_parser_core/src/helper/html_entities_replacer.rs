use color_eyre::Result;
use htmlentity::entity::{ICodedDataTrait, decode};
pub fn decode_html_entities(input: &str) -> Result<String> {
  let decoded = decode(input.as_bytes());

  decoded.to_string().map_err(|e| color_eyre::eyre::eyre!("Failed to decode HTML entities: {}", e.to_string()))
}
