use serde::{Serialize, de::DeserializeOwned};

const MAGIC: &[u8; 8] = b"MLCSYM\0\0";

pub fn to_binary<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend(super::FORMAT_VERSION.to_le_bytes());
    // Tagged enum contents deserialize through Serde's self-describing model.
    // Preserve field names so struct variants remain compatible with Temp nodes.
    value
        .serialize(&mut rmp_serde::Serializer::new(&mut bytes).with_struct_map())
        .map_err(|error| error.to_string())?;
    Ok(bytes)
}

pub fn from_binary<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, String> {
    if bytes.len() < 12 || &bytes[..8] != MAGIC {
        return Err("Invalid binary artifact header".into());
    }
    if bytes[8..12] != super::FORMAT_VERSION.to_le_bytes() {
        return Err("Unsupported artifact format version".into());
    }
    let mut decoder = rmp_serde::Deserializer::new(std::io::Cursor::new(&bytes[12..]));
    let value = T::deserialize(&mut decoder).map_err(|error| error.to_string())?;
    if decoder.position() != (bytes.len() - 12) as u64 {
        return Err("Trailing bytes in binary artifact".into());
    }
    Ok(value)
}
