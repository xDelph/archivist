use super::{AuthIdentity, LocalAuthStore};
use crate::auth::SlackIdentityResponse;
use tempfile::tempdir;

#[tokio::test]
async fn local_auth_store_upserts_and_reloads_identities() {
    let tempdir = tempdir().expect("tempdir");
    let path = tempdir.path().join("auth-identities.json");
    let store = LocalAuthStore::open(&path).await.expect("store");

    store
        .upsert_identity(&SlackIdentityResponse {
            ok: true,
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Thomas".to_owned()),
            avatar_url: Some("https://images.example.com/avatar.png".to_owned()),
        })
        .await
        .expect("first upsert");
    store
        .upsert_identity(&SlackIdentityResponse {
            ok: true,
            slack_user_id: "U123".to_owned(),
            email: Some("thomas@example.com".to_owned()),
            display_name: Some("Tom".to_owned()),
            avatar_url: Some("https://images.example.com/avatar-2.png".to_owned()),
        })
        .await
        .expect("second upsert");

    let reopened = LocalAuthStore::open(&path).await.expect("reopened");
    let identities = reopened.identities().await;

    assert_eq!(
        identities,
        vec![AuthIdentity {
            slack_user_id: "U123".to_owned(),
            display_name: Some("Tom".to_owned()),
            avatar_url: Some("https://images.example.com/avatar-2.png".to_owned()),
        }]
    );
}
