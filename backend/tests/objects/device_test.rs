use backend::objects::revoked_payload;

#[test]
fn revoked_payload_contract() {
    let payload = revoked_payload();
    assert_eq!(payload["status"], "revoked");
    assert!(payload["message"].as_str().is_some());
}
