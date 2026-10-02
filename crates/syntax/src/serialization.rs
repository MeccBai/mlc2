//! TOML persistence for the existing Temp types, without altering their fields.
//! A reserved table represents serde's null/unit values, which TOML cannot express.
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;

pub fn to_toml<T: Serialize>(value: &T) -> Result<String, String> {
    let json = serde_json::to_value(value).map_err(|e| e.to_string())?;
    let encoded = encode(json)?;
    toml::to_string_pretty(&encoded).map_err(|e| e.to_string())
}

pub fn from_toml<T: DeserializeOwned>(text: &str) -> Result<T, String> {
    let encoded: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    serde_json::from_value(decode(encoded)?).map_err(|e| e.to_string())
}

fn encode(value: Value) -> Result<toml::Value, String> {
    Ok(match value {
        Value::Null => toml::Value::Table(toml::Table::from_iter([(
            "$none".into(),
            toml::Value::Boolean(true),
        )])),
        Value::Bool(v) => toml::Value::Boolean(v),
        Value::String(v) => toml::Value::String(v),
        Value::Number(v) => {
            if let Some(n) = v.as_i64() {
                toml::Value::Integer(n)
            } else if v.is_u64() {
                return Err("Integer exceeds TOML's signed 64-bit range".into());
            } else if let Some(n) = v.as_f64() {
                toml::Value::Float(n)
            } else {
                return Err("Unsupported TOML number".into());
            }
        }
        Value::Array(values) => {
            toml::Value::Array(values.into_iter().map(encode).collect::<Result<_, _>>()?)
        }
        Value::Object(values) => toml::Value::Table(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, encode(value)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}

fn decode(value: toml::Value) -> Result<Value, String> {
    Ok(match value {
        toml::Value::Table(values)
            if values.len() == 1 && values.get("$none") == Some(&toml::Value::Boolean(true)) =>
        {
            Value::Null
        }
        toml::Value::Table(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, decode(value)?)))
                .collect::<Result<_, String>>()?,
        ),
        toml::Value::Array(values) => {
            Value::Array(values.into_iter().map(decode).collect::<Result<_, _>>()?)
        }
        toml::Value::String(v) => Value::String(v),
        toml::Value::Integer(v) => Value::Number(v.into()),
        toml::Value::Float(v) => serde_json::Number::from_f64(v)
            .map(Value::Number)
            .ok_or("Non-finite TOML number")?,
        toml::Value::Boolean(v) => Value::Bool(v),
        toml::Value::Datetime(_) => {
            return Err("Datetime is not part of the compiler format".into());
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::out::TempModule;
    #[derive(Debug, Serialize, serde::Deserialize, PartialEq)]
    struct Document {
        module: TempModule,
    }

    #[test]
    fn recursive_temp_roundtrip_keeps_spans_and_optional_nodes() {
        let source = "import std::io; unit P<T> { pub a:T; }; func<T> f(x:T) -> T { if (true) { return x; } return x; } func main() { var a = [1,2]; var p = P<i32>{1}; var s = \"hello\"; for i in [0,2] { a[i] = i; } }";
        let tokens = crate::lexer::tokenize(source).unwrap();
        let (module, errors) = crate::parser::parse(&tokens.tokens, source.len());
        assert!(errors.is_empty(), "{errors:?}");
        let document = Document {
            module: module.unwrap(),
        };
        let text = to_toml(&document).unwrap();
        assert_eq!(from_toml::<Document>(&text).unwrap(), document);
    }
}
