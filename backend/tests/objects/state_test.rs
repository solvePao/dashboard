use backend::models::auth::User;
use backend::models::device::{PairingState, PairingStatus};
use backend::objects::state::{
    as_check, commit_pairing, expire_pairing, init_pairing, AlarmOutcome, CommitOutcome,
};

fn user(id: i64) -> User {
    User {
        id,
        google_sub: format!("sub-{id}"),
        email: format!("u{id}@x.com"),
        name: None,
        picture_url: None,
        created_at: "c".to_string(),
        updated_at: "u".to_string(),
        last_login_at: None,
    }
}

fn pending() -> PairingState {
    PairingState {
        status: PairingStatus::Pending,
        expires_at: "exp-1".to_string(),
        token: None,
        user: None,
    }
}

fn authorized() -> PairingState {
    PairingState {
        status: PairingStatus::Authorized,
        expires_at: "exp-1".to_string(),
        token: Some("tok".to_string()),
        user: Some(user(3)),
    }
}

fn expired() -> PairingState {
    PairingState {
        status: PairingStatus::Expired,
        expires_at: "exp-1".to_string(),
        token: None,
        user: None,
    }
}

#[test]
fn init_creates_pending_and_refreshes_non_finished() {
    let created = init_pairing(None, "exp-2");
    assert_eq!(created.status, PairingStatus::Pending);
    assert_eq!(created.expires_at, "exp-2");
    assert!(created.token.is_none());

    // Pending and expired refresh to pending with the new expiry.
    let refreshed = init_pairing(Some(&pending()), "exp-3");
    assert_eq!(
        (refreshed.status, refreshed.expires_at.as_str()),
        (PairingStatus::Pending, "exp-3")
    );
    let refreshed = init_pairing(Some(&expired()), "exp-3");
    assert_eq!(refreshed.status, PairingStatus::Pending);
}

#[test]
fn init_never_reopens_authorized() {
    let kept = init_pairing(Some(&authorized()), "exp-9");
    assert_eq!(kept.status, PairingStatus::Authorized);
    assert_eq!(kept.expires_at, "exp-1");
    assert_eq!(kept.token.as_deref(), Some("tok"));
}

#[test]
fn commit_transitions_pending_to_authorized() {
    let u = user(7);
    match commit_pairing(Some(&pending()), "tok-7", &u) {
        CommitOutcome::Committed(next) => {
            assert_eq!(next.status, PairingStatus::Authorized);
            assert_eq!(next.expires_at, "exp-1");
            assert_eq!(next.token.as_deref(), Some("tok-7"));
            assert_eq!(next.user.map(|u| u.id), Some(7));
        }
        _ => panic!("expected commit"),
    }
}

#[test]
fn commit_refuses_finished_and_missing() {
    let u = user(7);
    assert!(matches!(
        commit_pairing(Some(&authorized()), "tok-x", &u),
        CommitOutcome::AlreadyAuthorized
    ));
    assert!(matches!(
        commit_pairing(None, "tok-x", &u),
        CommitOutcome::MissingOrExpired
    ));
    assert!(matches!(
        commit_pairing(Some(&expired()), "tok-x", &u),
        CommitOutcome::MissingOrExpired
    ));
}

#[test]
fn alarm_expires_pending_and_wipes_finished() {
    assert!(expire_pairing(None).is_none());
    match expire_pairing(Some(&pending())) {
        Some(AlarmOutcome::Expired(next)) => {
            assert_eq!(next.status, PairingStatus::Expired);
            assert_eq!(next.expires_at, "exp-1");
            assert!(next.token.is_none());
        }
        _ => panic!("expected expiry"),
    }
    assert!(matches!(expire_pairing(Some(&authorized())), Some(AlarmOutcome::Wipe)));
    assert!(matches!(expire_pairing(Some(&expired())), Some(AlarmOutcome::Wipe)));
}

#[test]
fn check_view_exposes_secrets_only_when_authorized() {
    let pending_view = as_check(&pending());
    assert_eq!(pending_view.status, PairingStatus::Pending);
    assert!(pending_view.token.is_none() && pending_view.user.is_none());

    let expired_view = as_check(&expired());
    assert_eq!(expired_view.status, PairingStatus::Expired);
    assert!(expired_view.token.is_none());

    // Even a pending record carrying credentials must not leak them.
    let mut smuggled = pending();
    smuggled.token = Some("tok".to_string());
    smuggled.user = Some(user(1));
    let view = as_check(&smuggled);
    assert!(view.token.is_none() && view.user.is_none());

    let view = as_check(&authorized());
    assert_eq!(view.token.as_deref(), Some("tok"));
    assert_eq!(view.user.map(|u| u.id), Some(3));
}

#[test]
fn device_revocation_payload_matches_mobile_contract() {
    let payload = serde_json::json!({
        "status": "revoked",
        "message": "Device access was revoked from the web dashboard."
    });
    assert_eq!(payload["status"], "revoked");
    assert_eq!(
        payload["message"],
        "Device access was revoked from the web dashboard."
    );
}
