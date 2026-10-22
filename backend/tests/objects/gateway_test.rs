use backend::common::{gateway_device_name, gateway_pairing_name, gateway_user_name};
use backend::objects::envelope;

#[test]
fn envelope_flattens_payload_and_tags_channel() {
    let msg = envelope("walkie", Some("grp-1"), &serde_json::json!({"type": "snapshot", "clients": []}));
    assert_eq!(msg["v"], 1);
    assert_eq!(msg["channel"], "walkie");
    assert_eq!(msg["group_id"], "grp-1");
    assert_eq!(msg["type"], "snapshot");
}

#[test]
fn envelope_without_group_omits_group_id() {
    let msg = envelope("device", None, &serde_json::json!({"status": "revoked"}));
    assert_eq!(msg["channel"], "device");
    assert_eq!(msg["status"], "revoked");
    assert!(msg.get("group_id").is_none());
}

#[test]
fn envelope_wraps_non_object_payload() {
    let msg = envelope("system", None, &serde_json::json!("ping"));
    assert_eq!(msg["payload"], "ping");
}

#[test]
fn gateway_names_are_namespaced_per_identity() {
    assert_eq!(gateway_device_name("tok"), "device:tok");
    assert_eq!(gateway_user_name(7), "user:7");
    assert_eq!(gateway_pairing_name("sess"), "pairing:sess");
    assert_ne!(gateway_device_name("7"), gateway_user_name(7));
}
