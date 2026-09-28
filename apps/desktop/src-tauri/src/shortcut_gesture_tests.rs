use super::*;
use crate::shortcut_edge::ShortcutEdge;

fn configured(mode: Mode) -> (Arc<Gestures>, ShortcutEdge) {
    let gestures = Arc::new(Gestures::default());
    gestures.configure().unwrap().commit(1, mode);
    (gestures, ShortcutEdge::default())
}
#[test]
fn hold_release_before_claim_cannot_launch() {
    let (gestures, edge) = configured(Mode::Hold);
    let press = gestures.edge(1, &edge, true).unwrap();
    let release = gestures.edge(1, &edge, false).unwrap();
    assert_eq!(press.id, release.id);
    assert_eq!(release.phase, Phase::Released);
    assert!(gestures.claim(&press.id).is_err());
}
#[test]
fn hold_release_during_setup_revokes_claimed_permit() {
    let (gestures, edge) = configured(Mode::Hold);
    let press = gestures.edge(1, &edge, true).unwrap();
    let permit = gestures.claim(&press.id).unwrap();
    assert!(permit.check().is_ok());
    gestures.edge(1, &edge, false).unwrap();
    assert!(permit.check().is_err());
}
#[test]
fn toggle_release_keeps_start_valid_but_each_press_is_single_use() {
    let (gestures, edge) = configured(Mode::Toggle);
    let press = gestures.edge(1, &edge, true).unwrap();
    for _ in 0..20 {
        assert!(gestures.edge(1, &edge, true).is_none());
    }
    gestures.edge(1, &edge, false).unwrap();
    let permit = gestures.claim(&press.id).unwrap();
    assert!(permit.check().is_ok());
    assert!(gestures.claim(&press.id).is_err());
    assert!(gestures.edge(1, &edge, false).is_none());
}
#[test]
fn rapid_repress_invalidates_previous_pending_or_claimed_start() {
    let (gestures, edge) = configured(Mode::Toggle);
    let first = gestures.edge(1, &edge, true).unwrap();
    let permit = gestures.claim(&first.id).unwrap();
    gestures.edge(1, &edge, false).unwrap();
    let next = gestures.edge(1, &edge, true).unwrap();
    assert_ne!(first.id, next.id);
    assert!(gestures.claim(&first.id).is_err());
    assert!(permit.check().is_err());
    assert!(gestures.claim(&next.id).unwrap().check().is_ok());
}
#[test]
fn cancellation_revokes_claim_and_cannot_revive_on_release() {
    let (gestures, edge) = configured(Mode::Hold);
    let press = gestures.edge(1, &edge, true).unwrap();
    let permit = gestures.claim(&press.id).unwrap();
    gestures.invalidate();
    assert!(permit.check().is_err());
    assert!(gestures.edge(1, &edge, false).is_none());
    assert!(gestures.claim(&press.id).is_err());
}
#[test]
fn held_binding_blocks_configuration_even_if_inactive() {
    let (gestures, edge) = configured(Mode::Toggle);
    assert!(gestures.edge(2, &edge, true).is_none());
    assert!(gestures.configure().is_err());
    assert!(gestures.edge(2, &edge, false).is_none());
    gestures.configure().unwrap().commit(2, Mode::Hold);
    let press = gestures.edge(2, &edge, true).unwrap();
    assert_eq!(press.mode, Mode::Hold);
}
#[test]
fn press_during_configuration_tracks_key_without_emitting_or_reinterpreting() {
    let (gestures, edge) = configured(Mode::Toggle);
    let config = gestures.configure().unwrap();
    assert!(gestures.edge(2, &edge, true).is_none());
    config.commit(2, Mode::Hold);
    assert!(gestures.edge(2, &edge, true).is_none());
    assert!(gestures.edge(2, &edge, false).is_none());
    let press = gestures.edge(2, &edge, true).unwrap();
    assert_eq!(press.mode, Mode::Hold);
    assert_eq!(press.phase, Phase::Pressed);
}
#[test]
fn failed_configuration_restores_previous_mode_and_binding() {
    let (gestures, edge) = configured(Mode::Toggle);
    drop(gestures.configure().unwrap());
    let event = gestures.edge(1, &edge, true).unwrap();
    assert_eq!(event.mode, Mode::Toggle);
    assert!(gestures.claim("garbage").is_err());
    assert!(gestures.claim("0").is_err());
}
#[test]
fn event_payload_and_mode_defaults_match_ipc_contract() {
    assert_eq!(Mode::parse(None), Ok(Mode::Toggle));
    assert_eq!(Mode::parse(Some("hold")), Ok(Mode::Hold));
    assert!(Mode::parse(Some("typo")).is_err());
    let (gestures, edge) = configured(Mode::Hold);
    let event = gestures.edge(1, &edge, true).unwrap();
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        serde_json::json!({"id":"1", "phase":"pressed", "mode":"hold"})
    );
}
