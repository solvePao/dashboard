use backend::common::DeviceLifecycleKind;
use backend::services::pairing::map_commit_status;
use backend::services::store::{DeviceNotifier, HubNotifier, NoNotify};

#[test]
fn commit_status_mapping() {
    assert!(map_commit_status(200).unwrap());
    assert!(map_commit_status(201).unwrap());
    // The object refused: the pairing already finished (409 race).
    assert!(!map_commit_status(409).unwrap());
    let err = map_commit_status(404).unwrap_err();
    assert_eq!(err.status, 404);
    assert_eq!(err.message, "Pending session not found or expired.");
}

#[tokio::test]
async fn no_notify_implements_notifiers_without_panic() {
    let no_notify = NoNotify;
    no_notify.notify_revoked("tok-123").await;
    no_notify.notify_hub(42, DeviceLifecycleKind::Authorized, None).await;
    no_notify.notify_hub(42, DeviceLifecycleKind::Revoked, Some("tok-123")).await;
    no_notify.notify_hub(42, DeviceLifecycleKind::Disconnected, Some("tok-123")).await;
}
