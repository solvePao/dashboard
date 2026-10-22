use serde::Deserialize;
use worker::*;

pub fn text_or_null(value: Option<&str>) -> D1Type<'_> {
    value.map(D1Type::Text).unwrap_or(D1Type::Null)
}

/// Binds a user row id. `D1Type::Integer` only holds an `i32` (the JS
/// number binding has no BigInt), so ids outside that range fail closed
/// instead of silently truncating.
pub fn user_id_param(user_id: i64) -> Result<D1Type<'static>> {
    let id = i32::try_from(user_id)
        .map_err(|_| Error::RustError("User id out of range.".to_string()))?;
    Ok(D1Type::Integer(id))
}

/// Reads a SQLite-stored boolean: native JSON bools pass through,
/// integers follow C truthiness (nonzero is true), and anything else
/// (null, text, floats) is false. Used via `deserialize_with` on
/// integer-backed flag columns.
pub fn sqlite_bool<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> std::result::Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(v.as_bool()
        .unwrap_or_else(|| v.as_i64().is_some_and(|n| n != 0)))
}

