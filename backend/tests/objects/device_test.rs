use backend::objects::{revoked_payload, walkie_activate_payload, widgets_updated_payload};

#[test]
fn revoked_payload_contract() {
    let payload = revoked_payload();
    assert_eq!(payload["status"], "revoked");
    assert!(payload["message"].as_str().is_some());
}

#[test]
fn widgets_updated_payload_contract() {
    // The mobile socket stays open on this status (unlike revocation),
    // so the payload must be distinguishable by `status` alone.
    let payload = widgets_updated_payload();
    assert_eq!(payload["status"], "widgets_updated");
    assert!(payload["message"].as_str().is_some());
    assert_ne!(payload["status"], revoked_payload()["status"]);
}

#[test]
fn walkie_activate_payload_contract() {
    let payload = walkie_activate_payload("grp-123");
    assert_eq!(payload["status"], "walkie_activate");
    assert_eq!(payload["group_id"], "grp-123");
    assert!(payload["message"].as_str().is_some());
    assert_ne!(payload["status"], widgets_updated_payload()["status"]);
}
