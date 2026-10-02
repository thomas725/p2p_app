use super::*;
use crate::behavior::CallSignal;

#[test]
fn outgoing_session_starts_calling() {
    let s = CallSession::outgoing("peer-1");
    assert!(!s.call_id.is_empty());
    assert_eq!(s.peer_id, "peer-1");
    assert_eq!(s.state, CallState::Calling);
    assert!(s.is_active());
}

#[test]
fn incoming_session_starts_ringing() {
    let s = CallSession::incoming("c1", "peer-1");
    assert_eq!(s.call_id, "c1");
    assert_eq!(s.peer_id, "peer-1");
    assert_eq!(s.state, CallState::Ringing);
    assert!(s.is_active());
}

#[test]
fn accept_lands_the_call() {
    let mut s = CallSession::outgoing("peer-1");
    let sig = CallSignal::new("accept", s.call_id.clone());
    assert!(s.on_signal(&sig));
    assert_eq!(s.state, CallState::InCall);
}

#[test]
fn reject_ends_the_call() {
    let mut s = CallSession::outgoing("peer-1");
    let sig = CallSignal::new("reject", s.call_id.clone());
    assert!(s.on_signal(&sig));
    assert_eq!(s.state, CallState::Ended);
    assert!(!s.is_active());
}

#[test]
fn end_tears_down_an_active_call() {
    let mut s = CallSession::incoming("c1", "peer-1");
    assert!(s.on_signal(&CallSignal::new("accept", "c1".into())));
    assert_eq!(s.state, CallState::InCall);
    assert!(s.on_signal(&CallSignal::new("end", "c1".into())));
    assert_eq!(s.state, CallState::Ended);
    assert!(!s.is_active());
}

#[test]
fn ending_already_ended_is_idempotent() {
    let mut s = CallSession::outgoing("peer-1");
    assert!(s.on_signal(&CallSignal::new("reject", s.call_id.clone())));
    assert!(!s.is_active());
    assert!(s.on_signal(&CallSignal::new("end", s.call_id.clone())));
    assert_eq!(s.state, CallState::Ended);
}

#[test]
fn foreign_call_id_is_ignored() {
    let mut s = CallSession::incoming("c1", "peer-1");
    assert!(!s.on_signal(&CallSignal::new("end", "c2".into())));
    assert_eq!(s.state, CallState::Ringing);
    assert!(!s.on_signal(&CallSignal::new("accept", String::new())));
    assert_eq!(s.state, CallState::Ringing);
}

#[test]
fn invite_is_handled_by_incoming_never_on_signal() {
    // The caller mints the id; an invite is adopted via `CallSession::incoming`,
    // so a stray invite addressed to an existing session is a no-op.
    let mut s = CallSession::incoming("c1", "peer-1");
    assert!(!s.on_signal(&CallSignal::new("invite", "c1".into())));
    assert_eq!(s.state, CallState::Ringing);
}

#[test]
fn default_session_is_the_no_call_sentinel() {
    let s = CallSession::default();
    assert_eq!(s.state, CallState::Ended);
    assert!(!s.is_active());
}

#[test]
fn equal_sessions_are_equal() {
    let mut a = CallSession::incoming("c1", "peer-1");
    let b = CallSession::incoming("c1", "peer-1");
    assert_eq!(a, b);
    a.on_signal(&CallSignal::new("accept", "c1".into()));
    assert_ne!(a, b);
}