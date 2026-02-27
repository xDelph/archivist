use crate::slack::types::{EventCallback, SlackEnvelope, SlackEvent, UrlVerification};

// ── Fixtures ──────────────────────────────────────────────────────────────────

const URL_VERIFICATION: &str = r#"{
    "type": "url_verification",
    "challenge": "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tBr7nud3XIVENOF2RZs",
    "token": "ignored"
}"#;

const EVENT_MESSAGE: &str = r#"{
    "type": "event_callback",
    "team_id": "T123",
    "api_app_id": "A123",
    "event_id": "Ev123",
    "event_time": 1700000000,
    "event": {
        "type": "message",
        "channel": "C123",
        "user": "U123",
        "text": "hello world",
        "ts": "1700000000.000200",
        "thread_ts": null,
        "subtype": null
    }
}"#;

const EVENT_THREAD_REPLY: &str = r#"{
    "type": "event_callback",
    "team_id": "T123",
    "api_app_id": "A123",
    "event_id": "Ev124",
    "event_time": 1700000001,
    "event": {
        "type": "message",
        "channel": "C123",
        "user": "U456",
        "text": "replying in thread",
        "ts": "1700000001.000100",
        "thread_ts": "1700000000.000200"
    }
}"#;

const EVENT_BOT_MESSAGE: &str = r#"{
    "type": "event_callback",
    "team_id": "T123",
    "api_app_id": "A123",
    "event_id": "Ev125",
    "event_time": 1700000002,
    "event": {
        "type": "message",
        "channel": "C123",
        "text": "I am a bot",
        "ts": "1700000002.000100",
        "subtype": "bot_message"
    }
}"#;

const EVENT_REACTION_ADDED: &str = r#"{
    "type": "event_callback",
    "team_id": "T123",
    "api_app_id": "A123",
    "event_id": "Ev126",
    "event_time": 1700000003,
    "event": {
        "type": "reaction_added",
        "reaction": "thumbsup",
        "user": "U123",
        "item": {
            "type": "message",
            "channel": "C123",
            "ts": "1700000000.000200"
        },
        "event_ts": "1700000003.000100"
    }
}"#;

const EVENT_UNKNOWN_TYPE: &str = r#"{
    "type": "event_callback",
    "team_id": "T123",
    "api_app_id": "A123",
    "event_id": "Ev127",
    "event_time": 1700000004,
    "event": {
        "type": "app_mention",
        "channel": "C123",
        "ts": "1700000004.000100"
    }
}"#;

const UNKNOWN_ENVELOPE_TYPE: &str = r#"{
    "type": "app_rate_limited",
    "team_id": "T123",
    "minute_rate_limited": 1700000000
}"#;

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_url_verification_deserializes() {
    let env: SlackEnvelope = serde_json::from_str(URL_VERIFICATION).unwrap();
    match env {
        SlackEnvelope::UrlVerification(uv) => {
            assert_eq!(
                uv.challenge,
                "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tBr7nud3XIVENOF2RZs"
            );
        }
        other => panic!("expected UrlVerification, got {other:?}"),
    }
}

#[test]
fn test_url_verification_challenge_serializes() {
    let uv = UrlVerification {
        challenge: "abc123".into(),
    };
    let json = serde_json::to_string(&uv).unwrap();
    assert_eq!(json, r#"{"challenge":"abc123"}"#);
}

#[test]
fn test_message_event_deserializes() {
    let env: SlackEnvelope = serde_json::from_str(EVENT_MESSAGE).unwrap();
    let cb = assert_event_callback(env);
    assert_eq!(cb.event_id, "Ev123");
    assert_eq!(cb.team_id, "T123");
    match cb.event {
        SlackEvent::Message(m) => {
            assert_eq!(m.channel, "C123");
            assert_eq!(m.user.as_deref(), Some("U123"));
            assert_eq!(m.text.as_deref(), Some("hello world"));
            assert_eq!(m.ts, "1700000000.000200");
            assert!(m.thread_ts.is_none());
            assert!(m.subtype.is_none());
        }
        other => panic!("expected Message, got {other:?}"),
    }
}

#[test]
fn test_thread_reply_has_thread_ts() {
    let env: SlackEnvelope = serde_json::from_str(EVENT_THREAD_REPLY).unwrap();
    let cb = assert_event_callback(env);
    match cb.event {
        SlackEvent::Message(m) => {
            assert_eq!(m.thread_ts.as_deref(), Some("1700000000.000200"));
        }
        other => panic!("expected Message, got {other:?}"),
    }
}

#[test]
fn test_bot_message_has_subtype_and_no_user() {
    let env: SlackEnvelope = serde_json::from_str(EVENT_BOT_MESSAGE).unwrap();
    let cb = assert_event_callback(env);
    match cb.event {
        SlackEvent::Message(m) => {
            assert_eq!(m.subtype.as_deref(), Some("bot_message"));
            assert!(m.user.is_none());
        }
        other => panic!("expected Message, got {other:?}"),
    }
}

#[test]
fn test_reaction_added_deserializes() {
    let env: SlackEnvelope = serde_json::from_str(EVENT_REACTION_ADDED).unwrap();
    let cb = assert_event_callback(env);
    match cb.event {
        SlackEvent::ReactionAdded(r) => {
            assert_eq!(r.reaction, "thumbsup");
            assert_eq!(r.user, "U123");
            assert_eq!(r.item.channel, "C123");
            assert_eq!(r.item.ts, "1700000000.000200");
            assert_eq!(r.event_ts, "1700000003.000100");
        }
        other => panic!("expected ReactionAdded, got {other:?}"),
    }
}

#[test]
fn test_unknown_inner_event_is_ignored() {
    let env: SlackEnvelope = serde_json::from_str(EVENT_UNKNOWN_TYPE).unwrap();
    let cb = assert_event_callback(env);
    assert!(matches!(cb.event, SlackEvent::Unknown));
}

#[test]
fn test_unknown_envelope_type_is_ignored() {
    let env: SlackEnvelope = serde_json::from_str(UNKNOWN_ENVELOPE_TYPE).unwrap();
    assert!(matches!(env, SlackEnvelope::Unknown));
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn assert_event_callback(env: SlackEnvelope) -> EventCallback {
    match env {
        SlackEnvelope::EventCallback(cb) => *cb,
        other => panic!("expected EventCallback, got {other:?}"),
    }
}
