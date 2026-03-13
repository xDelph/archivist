use super::{
    build_channel_name_map, collect_channel_mention_ids, collect_user_mention_ids,
    decode_html_entities, render_slack_text,
};
use crate::user_store::SyncedUserRecord;
use domain::{Channel, ChannelKind};
use std::collections::HashMap;

#[test]
fn collect_mention_ids_finds_users_and_channels() {
    let users = collect_user_mention_ids(["Hello <@U123> and <@U456|thomas>"]);
    let channels = collect_channel_mention_ids(["See <#C123> and <#C456|general>"]);

    assert!(users.contains("U123"));
    assert!(users.contains("U456"));
    assert!(channels.contains("C123"));
    assert!(channels.contains("C456"));
}

#[test]
fn render_slack_text_resolves_mentions_and_entities() {
    let users = HashMap::from([(
        "U123".to_owned(),
        SyncedUserRecord {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Thomas".to_owned()),
            avatar_url: None,
            is_active: true,
        },
    )]);
    let channels = build_channel_name_map(&[Channel {
        id: "C123".to_owned(),
        name: Some("general".to_owned()),
        kind: ChannelKind::Public,
        is_archived: false,
    }]);

    assert_eq!(
        render_slack_text(
            "Hi <@U123>, check <#C123> --&gt; <!here> <!subteam^S123|platform>",
            &users,
            &channels,
        ),
        "Hi @Thomas, check #general --> @here @platform",
    );
}

#[test]
fn render_slack_text_falls_back_to_inline_labels() {
    assert_eq!(
        render_slack_text(
            "Ask <@U123|thomas> in <#C123|support>",
            &HashMap::new(),
            &HashMap::new(),
        ),
        "Ask @thomas in #support",
    );
}

#[test]
fn decode_html_entities_handles_common_named_and_numeric_entities() {
    assert_eq!(
        decode_html_entities("Tom &amp; Jerry &gt; Spike &#39;ok&#39; &#x27;great&#x27;"),
        "Tom & Jerry > Spike 'ok' 'great'",
    );
}
