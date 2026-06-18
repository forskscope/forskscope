use super::SessionState;

#[test]
fn session_state_serialises_and_deserialises() {
    let state = SessionState {
        tabs: vec![
            ("/old/a.rs".into(), "/new/a.rs".into()),
            ("/old/b.rs".into(), "/new/b.rs".into()),
        ],
    };
    let json = serde_json::to_string(&state).expect("serialise");
    let back: SessionState = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(back.tabs.len(), 2);
    assert_eq!(back.tabs[0].0, "/old/a.rs");
    assert_eq!(back.tabs[1].1, "/new/b.rs");
}

#[test]
fn empty_session_state_round_trips() {
    let state = SessionState::default();
    let json  = serde_json::to_string(&state).unwrap();
    let back: SessionState = serde_json::from_str(&json).unwrap();
    assert!(back.tabs.is_empty());
}
