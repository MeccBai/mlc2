use serde::Serialize;
use serde_json::Value;

pub fn content_hash(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

/// JSON is only an internal canonical hashing representation, not the disk format.
/// Object keys are ordered; source spans do not affect the semantic fingerprint.
pub fn semantic_hash(value: &impl Serialize) -> Result<String, String> {
    let mut value = serde_json::to_value(value).map_err(|e| e.to_string())?;
    strip_positions(&mut value);
    let bytes = serde_json::to_vec(&value).map_err(|e| e.to_string())?;
    Ok(content_hash(&bytes))
}

fn strip_positions(value: &mut Value) {
    match value {
        Value::Object(fields)
            if fields.len() == 3
                && fields.contains_key("start")
                && fields.contains_key("end")
                && fields.contains_key("context") =>
        {
            *value = Value::Null
        }
        Value::Object(fields) => {
            fields.retain(|key, _| !key.ends_with("_span"));
            fields.values_mut().for_each(strip_positions);
        }
        Value::Array(values) => values.iter_mut().for_each(strip_positions),
        _ => {}
    }
}
