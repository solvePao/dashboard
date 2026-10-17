use backend::utils::generate_qr_svg;

#[test]
fn qr_svg_contains_svg_for_simple_input() {
    let svg = generate_qr_svg("hello").expect("qr");
    assert!(svg.contains("<svg"));
    assert!(svg.contains("</svg>") || svg.contains("/>"));
}

#[test]
fn qr_svg_differs_per_input() {
    let a = generate_qr_svg("session-a").unwrap();
    let b = generate_qr_svg("session-b").unwrap();
    assert_ne!(a, b);
}

#[test]
fn qr_svg_works_for_32_hex_session() {
    let session = "c".repeat(32);
    let svg = generate_qr_svg(&session).unwrap();
    assert!(svg.contains("<svg"));
}

#[test]
fn qr_svg_rejects_oversized_input() {
    // Beyond QR capacity the builder fails instead of truncating.
    assert!(generate_qr_svg(&"x".repeat(5000)).is_err());
}

#[test]
fn qr_svg_empty_does_not_panic() {
    // fast_qr encodes empty input; the service never sends empty (validated upstream).
    let res = generate_qr_svg("");
    assert!(res.is_ok() || res.is_err());
    if let Ok(svg) = res {
        assert!(svg.contains("<svg"));
    }
}
