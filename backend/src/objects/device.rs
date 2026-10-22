/// Payload pushed to live device sockets on revocation. Pure so the
/// contract both sides depend on is unit-tested.
pub fn revoked_payload() -> serde_json::Value {
    serde_json::json!({
        "status": "revoked",
        "message": "Device access was revoked from the web dashboard."
    })
}

/// Payload pushed to live device sockets when the web dashboard saves
/// widgets. The socket stays open; the device re-fetches its widget set.
pub fn widgets_updated_payload() -> serde_json::Value {
    serde_json::json!({
        "status": "widgets_updated",
        "message": "Widget configuration was updated from the web dashboard."
    })
}

/// Payload pushed to live device sockets when an incoming walkie-talkie call/activation occurs.
pub fn walkie_activate_payload(group_id: &str) -> serde_json::Value {
    serde_json::json!({
        "status": "walkie_activate",
        "group_id": group_id,
        "message": "Incoming walkie-talkie call."
    })
}
