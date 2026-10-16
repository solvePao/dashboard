use backend::services::pairing::map_commit_status;

#[test]
fn commit_status_mapping() {
    assert!(map_commit_status(200).unwrap());
    // The object refused: the pairing already finished (409 race).
    assert!(!map_commit_status(409).unwrap());
    let err = map_commit_status(404).unwrap_err();
    assert_eq!(err.status, 404);
}
