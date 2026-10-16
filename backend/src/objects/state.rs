use crate::models::auth::User;
use crate::models::device::{CheckSessionResponse, PairingState};

/// Pure pairing state machine. The Durable Object handler is a thin
/// wrapper over these rules so they are unit-testable on any target.
/// Result of an authorize commit attempt.
#[allow(clippy::large_enum_variant)]
pub enum CommitOutcome {
    Committed(PairingState),
    AlreadyAuthorized,
    MissingOrExpired,
}

/// Result of the expiry alarm for a stored pairing.
#[allow(clippy::large_enum_variant)]
pub enum AlarmOutcome {
    /// A pending pairing becomes expired and is kept for status reads.
    Expired(PairingState),
    /// Finished pairings are wiped once the window closes.
    Wipe,
}

/// Starts (or refreshes) a pairing. A finished pairing is never reopened.
pub fn init_pairing(existing: Option<&PairingState>, expires_at: &str) -> PairingState {
    if let Some(pairing) = existing {
        if pairing.status == "authorized" {
            return pairing.clone();
        }
    }
    PairingState {
        status: "pending".to_string(),
        expires_at: expires_at.to_string(),
        token: None,
        user: None,
    }
}

/// Commits a minted token to a pending pairing.
pub fn commit_pairing(
    existing: Option<&PairingState>,
    token: &str,
    user: &User,
) -> CommitOutcome {
    let Some(pairing) = existing else {
        return CommitOutcome::MissingOrExpired;
    };
    match pairing.status.as_str() {
        "authorized" => CommitOutcome::AlreadyAuthorized,
        "pending" => CommitOutcome::Committed(PairingState {
            status: "authorized".to_string(),
            expires_at: pairing.expires_at.clone(),
            token: Some(token.to_string()),
            user: Some(user.clone()),
        }),
        _ => CommitOutcome::MissingOrExpired,
    }
}

/// Applies the expiry alarm. `None` means the object holds nothing.
pub fn expire_pairing(existing: Option<&PairingState>) -> Option<AlarmOutcome> {
    let pairing = existing?;
    match pairing.status.as_str() {
        "pending" => Some(AlarmOutcome::Expired(PairingState {
            status: "expired".to_string(),
            ..pairing.clone()
        })),
        _ => Some(AlarmOutcome::Wipe),
    }
}

/// Public view of a pairing. Token and user are only exposed once
/// authorized; pending and expired views never leak them.
pub fn as_check(pairing: &PairingState) -> CheckSessionResponse {
    let authorized = pairing.status == "authorized";
    CheckSessionResponse {
        status: pairing.status.clone(),
        token: if authorized {
            pairing.token.clone()
        } else {
            None
        },
        user: if authorized {
            pairing.user.clone()
        } else {
            None
        },
    }
}
