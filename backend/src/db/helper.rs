use worker::*;

pub fn text_or_null(value: Option<&str>) -> D1Type<'_> {
    value.map(D1Type::Text).unwrap_or(D1Type::Null)
}

