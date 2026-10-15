use fast_qr::convert::{svg::SvgBuilder, Builder, Shape};
use fast_qr::qr::QRBuilder;

pub fn generate_qr_svg(data: &str) -> std::result::Result<String, String> {
    let qrcode = QRBuilder::new(data)
        .build()
        .map_err(|e| format!("Failed to build QR: {e:?}"))?;

    let mut builder = SvgBuilder::default();
    builder.shape(Shape::Square);
    let svg = builder.to_str(&qrcode);

    Ok(svg)
}

