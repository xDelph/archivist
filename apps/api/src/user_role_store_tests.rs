use super::{ADMIN_ROLE, LocalUserRoleStore};

#[tokio::test]
async fn local_user_role_store_grants_lists_and_revokes_roles() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let path = tempdir.path().join("user-roles.json");
    let store = LocalUserRoleStore::open(&path)
        .await
        .expect("user role store");
    let role_store = super::UserRoleStore::from(store.clone());

    assert!(
        role_store
            .grant_role("U123", ADMIN_ROLE)
            .await
            .expect("grant role")
    );
    assert!(
        !role_store
            .grant_role("U123", " Admin ")
            .await
            .expect("duplicate role")
    );
    assert_eq!(
        role_store.list_roles("U123").await.expect("list roles"),
        vec![ADMIN_ROLE.to_owned()]
    );
    let reopened = LocalUserRoleStore::open(&path)
        .await
        .expect("reopened user role store");
    assert!(
        super::UserRoleStore::from(reopened.clone())
            .has_role("U123", ADMIN_ROLE)
            .await
            .expect("has role")
    );
    assert!(
        role_store
            .revoke_role("U123", ADMIN_ROLE)
            .await
            .expect("revoke role")
    );
    assert_eq!(
        role_store.list_roles("U123").await.expect("list roles"),
        Vec::<String>::new()
    );
    assert!(
        !super::UserRoleStore::from(reopened)
            .has_role("U123", "moderator")
            .await
            .expect("missing role")
    );
    assert!(
        !super::UserRoleStore::from(store)
            .has_role("U123", "   ")
            .await
            .expect("blank role lookup")
    );
}

#[tokio::test]
async fn local_user_role_store_ignores_blank_roles() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let store = LocalUserRoleStore::open(tempdir.path().join("user-roles.json"))
        .await
        .expect("user role store");
    let role_store = super::UserRoleStore::from(store.clone());

    assert!(
        !role_store
            .grant_role("U123", "   ")
            .await
            .expect("blank role")
    );
    assert_eq!(
        role_store.list_roles("U123").await.expect("list roles"),
        Vec::<String>::new()
    );
}
