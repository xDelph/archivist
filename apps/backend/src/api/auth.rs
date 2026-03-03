use std::env;

use bytes::Bytes;
use chrono::{Duration, Utc};
use http::StatusCode;
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use tokio::sync::OnceCell;
use tracing::{error, warn};
use uuid::Uuid;
use vercel_runtime::{Error, Request, Response, ResponseBody};

use crate::db::pool::create_pool;
use crate::security::{
    email_lookup_hash, encrypt_email, generate_token, hash_password, normalize_email, sha256_hex,
    verify_password,
};

static POOL: OnceCell<PgPool> = OnceCell::const_new();

const SESSION_TTL_DAYS: i64 = 30;
const RESET_TOKEN_TTL_MINUTES: i64 = 30;
const INTERNAL_SECRET_HEADER: &str = "x-archivist-internal-secret";
const SESSION_HEADER: &str = "x-archivist-session";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthContext {
    pub account_id: Uuid,
    pub slack_user_id: String,
    pub is_anonymous: bool,
    pub display_name: String,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgotPasswordRequest {
    email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreferencesRequest {
    is_anonymous: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthSuccessResponse {
    ok: bool,
    session_token: String,
    user: AuthUserResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthUserResponse {
    account_id: String,
    slack_user_id: String,
    is_anonymous: bool,
    display_name: String,
    avatar_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MeResponse {
    ok: bool,
    user: AuthUserResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenericOkResponse {
    ok: bool,
}

#[derive(Debug)]
struct AccountRow {
    account_id: Uuid,
    slack_user_id: String,
    password_hash: String,
    is_anonymous: bool,
    display_name: String,
    avatar_url: String,
    account_disabled_at: Option<chrono::DateTime<Utc>>,
    slack_is_active: bool,
    slack_is_deleted: bool,
}

async fn pool() -> Result<&'static PgPool, Error> {
    POOL.get_or_try_init(|| async {
        let url = env::var("DATABASE_URL").unwrap_or_default();
        create_pool(&url)
            .await
            .map_err(|e| Error::from(e.to_string()))
    })
    .await
}

pub async fn handler(req: Request) -> Result<Response<ResponseBody>, Error> {
    let method = req.method().to_string();
    let path = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or("").to_owned();
    match tokio::spawn(async move { handle_request(req).await }).await {
        Ok(Ok(resp)) => Ok(resp),
        Ok(Err(err)) => {
            error!(method, path, query, error = %err, "auth handler failed");
            internal_error_response()
        }
        Err(join_err) => {
            error!(
                method,
                path,
                query,
                is_panic = join_err.is_panic(),
                error = %join_err,
                "auth handler task crashed"
            );
            internal_error_response()
        }
    }
}

async fn handle_request(req: Request) -> Result<Response<ResponseBody>, Error> {
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    let req = http::Request::from_parts(parts, bytes);
    let (parts, body) = process(pool().await?, req).await?.into_parts();
    Ok(Response::from_parts(parts, ResponseBody::from(body)))
}

pub(crate) async fn process(
    pool: &PgPool,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    if let Some(resp) = ensure_internal_secret(&req) {
        return Ok(resp);
    }

    let raw_path = req.uri().path();
    let path = raw_path.trim_end_matches('/');
    let method = req.method().clone();

    match (method, path) {
        (http::Method::POST, "/api/auth/register") => register(pool, req).await,
        (http::Method::POST, "/api/auth/login") => login(pool, req).await,
        (http::Method::POST, "/api/auth/logout") => logout(pool, &req).await,
        (http::Method::GET, "/api/auth/me") => me(pool, &req).await,
        (http::Method::POST, "/api/auth/change-password") => change_password(pool, req).await,
        (http::Method::PATCH, "/api/auth/preferences")
        | (http::Method::POST, "/api/auth/preferences") => update_preferences(pool, req).await,
        (http::Method::POST, "/api/auth/password/forgot") => forgot_password(pool, req).await,
        (http::Method::POST, "/api/auth/password/reset") => reset_password(pool, req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Bytes::new())?),
    }
}

pub(crate) async fn authorize_request(
    pool: &PgPool,
    req: &http::Request<Bytes>,
) -> Result<AuthContext, Response<Bytes>> {
    if let Some(resp) = ensure_internal_secret(req) {
        return Err(resp);
    }
    let Some(raw_session) = session_token_from_headers(req) else {
        return Err(error_response(StatusCode::UNAUTHORIZED, "missing session"));
    };
    let session_hash = sha256_hex(&raw_session);

    let row = sqlx::query(
        r#"
        SELECT
            a.id AS account_id,
            a.slack_user_id,
            a.is_anonymous,
            COALESCE(u.display_name, a.slack_user_id, '') AS display_name,
            COALESCE(u.avatar_url, '') AS avatar_url,
            a.disabled_at AS account_disabled_at,
            u.is_active AS slack_is_active,
            u.is_deleted AS slack_is_deleted
        FROM auth_sessions s
        JOIN auth_accounts a ON a.id = s.account_id
        JOIN users u ON u.user_id = a.slack_user_id
        WHERE s.session_token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
        LIMIT 1
        "#,
    )
    .bind(session_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;

    let Some(row) = row else {
        return Err(error_response(StatusCode::UNAUTHORIZED, "invalid session"));
    };

    let account_id: Uuid = row
        .try_get("account_id")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let slack_user_id: String = row
        .try_get("slack_user_id")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let is_anonymous: bool = row
        .try_get("is_anonymous")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let display_name: String = row
        .try_get("display_name")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let avatar_url: String = row
        .try_get("avatar_url")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let account_disabled_at: Option<chrono::DateTime<Utc>> =
        row.try_get("account_disabled_at")
            .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let slack_is_active: bool = row
        .try_get("slack_is_active")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;
    let slack_is_deleted: bool = row
        .try_get("slack_is_deleted")
        .map_err(|_| error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal server error"))?;

    if account_disabled_at.is_some() || !slack_is_active || slack_is_deleted {
        let _ = revoke_all_sessions(pool, account_id, "slack-deactivated").await;
        let _ = sqlx::query(
            r#"
            UPDATE auth_accounts
            SET disabled_at = COALESCE(disabled_at, NOW()),
                disabled_reason = COALESCE(disabled_reason, 'slack-deactivated'),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(account_id)
        .execute(pool)
        .await;
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            "account is not active",
        ));
    }

    Ok(AuthContext {
        account_id,
        slack_user_id,
        is_anonymous,
        display_name,
        avatar_url,
    })
}

async fn register(pool: &PgPool, req: http::Request<Bytes>) -> Result<Response<Bytes>, Error> {
    let payload: RegisterRequest = parse_json(req.body())?;
    if let Some(resp) = validate_password_policy(&payload.password) {
        return Ok(resp);
    }
    let normalized_email = normalize_email(&payload.email);
    if normalized_email.is_empty() {
        return Ok(error_response(StatusCode::BAD_REQUEST, "invalid email"));
    }
    let email_hash = match email_lookup_hash(&normalized_email) {
        Ok(v) => v,
        Err(err) => {
            error!(error = %err, "email lookup hash failed");
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };

    let slack_row = sqlx::query(
        r#"
        SELECT user_id, display_name, avatar_url, is_active, is_deleted
        FROM users
        WHERE email_lookup_hash = $1
        LIMIT 1
        "#,
    )
    .bind(&email_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    let Some(slack_row) = slack_row else {
        return Ok(error_response(
            StatusCode::FORBIDDEN,
            "email is not eligible",
        ));
    };
    let slack_user_id: String = slack_row.try_get("user_id").map_err(|e| Error::from(e.to_string()))?;
    let display_name: String = slack_row
        .try_get("display_name")
        .map_err(|e| Error::from(e.to_string()))?;
    let avatar_url: String = slack_row
        .try_get("avatar_url")
        .map_err(|e| Error::from(e.to_string()))?;
    let slack_is_active: bool = slack_row
        .try_get("is_active")
        .map_err(|e| Error::from(e.to_string()))?;
    let slack_is_deleted: bool = slack_row
        .try_get("is_deleted")
        .map_err(|e| Error::from(e.to_string()))?;
    if !slack_is_active || slack_is_deleted {
        return Ok(error_response(
            StatusCode::FORBIDDEN,
            "email is not eligible",
        ));
    }

    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM auth_accounts WHERE email_lookup_hash = $1",
    )
    .bind(&email_hash)
    .fetch_one(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    if existing > 0 {
        return Ok(error_response(
            StatusCode::CONFLICT,
            "account already exists",
        ));
    }

    let password_hash = match hash_password(&payload.password) {
        Ok(v) => v,
        Err(err) => {
            error!(error = %err, "password hash failed");
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };
    let encrypted_email = match encrypt_email(&normalized_email) {
        Ok(v) => v,
        Err(err) => {
            error!(error = %err, "email encryption failed");
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };

    let account_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO auth_accounts (
            id,
            slack_user_id,
            email_ciphertext,
            email_lookup_hash,
            password_hash
        )
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(account_id)
    .bind(&slack_user_id)
    .bind(encrypted_email)
    .bind(&email_hash)
    .bind(password_hash)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;

    let (session_token, _) = create_session(pool, account_id).await?;
    let (profile_name, profile_avatar) = account_profile(display_name.as_str(), avatar_url.as_str());
    json_ok(&AuthSuccessResponse {
        ok: true,
        session_token,
        user: AuthUserResponse {
            account_id: account_id.to_string(),
            slack_user_id,
            is_anonymous: false,
            display_name: profile_name,
            avatar_url: profile_avatar,
        },
    })
}

async fn login(pool: &PgPool, req: http::Request<Bytes>) -> Result<Response<Bytes>, Error> {
    let payload: LoginRequest = parse_json(req.body())?;
    let normalized_email = normalize_email(&payload.email);
    if normalized_email.is_empty() {
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid credentials",
        ));
    }
    let email_hash = match email_lookup_hash(&normalized_email) {
        Ok(v) => v,
        Err(err) => {
            error!(error = %err, "email lookup hash failed");
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };

    let account = fetch_account_by_email_hash(pool, &email_hash).await?;
    let Some(account) = account else {
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid credentials",
        ));
    };
    if account.account_disabled_at.is_some() || !account.slack_is_active || account.slack_is_deleted {
        revoke_all_sessions(pool, account.account_id, "slack-deactivated").await?;
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "account is not active",
        ));
    }
    if !verify_password(&payload.password, &account.password_hash) {
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid credentials",
        ));
    }

    let (session_token, _) = create_session(pool, account.account_id).await?;
    let (profile_name, profile_avatar) =
        account_profile(account.display_name.as_str(), account.avatar_url.as_str());
    json_ok(&AuthSuccessResponse {
        ok: true,
        session_token,
        user: AuthUserResponse {
            account_id: account.account_id.to_string(),
            slack_user_id: account.slack_user_id,
            is_anonymous: account.is_anonymous,
            display_name: profile_name,
            avatar_url: profile_avatar,
        },
    })
}

async fn me(pool: &PgPool, req: &http::Request<Bytes>) -> Result<Response<Bytes>, Error> {
    let context = match authorize_request(pool, req).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };
    let (profile_name, profile_avatar) =
        account_profile(context.display_name.as_str(), context.avatar_url.as_str());
    json_ok(&MeResponse {
        ok: true,
        user: AuthUserResponse {
            account_id: context.account_id.to_string(),
            slack_user_id: context.slack_user_id,
            is_anonymous: context.is_anonymous,
            display_name: profile_name,
            avatar_url: profile_avatar,
        },
    })
}

async fn logout(pool: &PgPool, req: &http::Request<Bytes>) -> Result<Response<Bytes>, Error> {
    if let Some(resp) = ensure_internal_secret(req) {
        return Ok(resp);
    }
    let Some(raw_session) = session_token_from_headers(req) else {
        return json_ok(&GenericOkResponse { ok: true });
    };
    let session_hash = sha256_hex(&raw_session);
    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW(),
            revoke_reason = 'logout'
        WHERE session_token_hash = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(session_hash)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    json_ok(&GenericOkResponse { ok: true })
}

async fn change_password(
    pool: &PgPool,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let context = match authorize_request(pool, &req).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };
    let payload: ChangePasswordRequest = parse_json(req.body())?;
    if let Some(resp) = validate_password_policy(&payload.new_password) {
        return Ok(resp);
    }

    let row = sqlx::query("SELECT password_hash FROM auth_accounts WHERE id = $1")
        .bind(context.account_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    let Some(row) = row else {
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "account not found",
        ));
    };
    let current_hash: String = row
        .try_get("password_hash")
        .map_err(|e| Error::from(e.to_string()))?;
    if !verify_password(&payload.current_password, &current_hash) {
        return Ok(error_response(
            StatusCode::UNAUTHORIZED,
            "invalid credentials",
        ));
    }
    let next_hash = hash_password(&payload.new_password).map_err(|e| Error::from(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE auth_accounts
        SET password_hash = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(context.account_id)
    .bind(next_hash)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;

    revoke_other_sessions(pool, context.account_id, session_token_from_headers(&req), "password-changed")
        .await?;
    json_ok(&GenericOkResponse { ok: true })
}

async fn update_preferences(
    pool: &PgPool,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let context = match authorize_request(pool, &req).await {
        Ok(v) => v,
        Err(resp) => return Ok(resp),
    };
    let payload: PreferencesRequest = parse_json(req.body())?;
    sqlx::query(
        r#"
        UPDATE auth_accounts
        SET is_anonymous = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(context.account_id)
    .bind(payload.is_anonymous)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;

    let (profile_name, profile_avatar) =
        account_profile(context.display_name.as_str(), context.avatar_url.as_str());
    json_ok(&MeResponse {
        ok: true,
        user: AuthUserResponse {
            account_id: context.account_id.to_string(),
            slack_user_id: context.slack_user_id,
            is_anonymous: payload.is_anonymous,
            display_name: profile_name,
            avatar_url: profile_avatar,
        },
    })
}

async fn forgot_password(
    pool: &PgPool,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let payload: ForgotPasswordRequest = parse_json(req.body())?;
    let normalized_email = normalize_email(&payload.email);
    if normalized_email.is_empty() {
        return json_ok(&GenericOkResponse { ok: true });
    }
    let email_hash = match email_lookup_hash(&normalized_email) {
        Ok(v) => v,
        Err(err) => {
            error!(error = %err, "email lookup hash failed");
            return Ok(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error",
            ));
        }
    };

    let row = sqlx::query("SELECT id FROM auth_accounts WHERE email_lookup_hash = $1 LIMIT 1")
        .bind(email_hash)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

    let mut reset_token_for_dev: Option<String> = None;
    if let Some(row) = row {
        let account_id: Uuid = row.try_get("id").map_err(|e| Error::from(e.to_string()))?;
        let token = generate_token();
        let token_hash = sha256_hex(&token);
        let expires_at = Utc::now() + Duration::minutes(RESET_TOKEN_TTL_MINUTES);
        sqlx::query(
            r#"
            INSERT INTO password_reset_tokens (id, account_id, token_hash, expires_at)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(account_id)
        .bind(token_hash)
        .bind(expires_at)
        .execute(pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?;

        if should_expose_reset_token_for_dev() {
            reset_token_for_dev = Some(token);
        } else {
            // Keep as placeholder until an email provider is wired.
            warn!("password reset requested, email delivery provider not configured");
        }
    }

    if let Some(token) = reset_token_for_dev {
        return json_ok(&serde_json::json!({
            "ok": true,
            "resetToken": token
        }));
    }
    json_ok(&GenericOkResponse { ok: true })
}

async fn reset_password(
    pool: &PgPool,
    req: http::Request<Bytes>,
) -> Result<Response<Bytes>, Error> {
    let payload: ResetPasswordRequest = parse_json(req.body())?;
    if let Some(resp) = validate_password_policy(&payload.new_password) {
        return Ok(resp);
    }
    let token_hash = sha256_hex(&payload.token);
    let row = sqlx::query(
        r#"
        SELECT id, account_id
        FROM password_reset_tokens
        WHERE token_hash = $1
          AND used_at IS NULL
          AND expires_at > NOW()
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    let Some(row) = row else {
        return Ok(error_response(
            StatusCode::BAD_REQUEST,
            "invalid reset token",
        ));
    };
    let reset_id: Uuid = row.try_get("id").map_err(|e| Error::from(e.to_string()))?;
    let account_id: Uuid = row
        .try_get("account_id")
        .map_err(|e| Error::from(e.to_string()))?;
    let password_hash = hash_password(&payload.new_password).map_err(|e| Error::from(e.to_string()))?;

    let mut tx = pool.begin().await.map_err(|e| Error::from(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE auth_accounts
        SET password_hash = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(account_id)
    .bind(password_hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1")
        .bind(reset_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW(),
            revoke_reason = 'password-reset'
        WHERE account_id = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(account_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    tx.commit().await.map_err(|e| Error::from(e.to_string()))?;
    json_ok(&GenericOkResponse { ok: true })
}

async fn fetch_account_by_email_hash(
    pool: &PgPool,
    email_hash: &str,
) -> Result<Option<AccountRow>, Error> {
    let row = sqlx::query(
        r#"
        SELECT
            a.id AS account_id,
            a.slack_user_id,
            a.password_hash,
            a.is_anonymous,
            COALESCE(u.display_name, a.slack_user_id, '') AS display_name,
            COALESCE(u.avatar_url, '') AS avatar_url,
            a.disabled_at AS account_disabled_at,
            u.is_active AS slack_is_active,
            u.is_deleted AS slack_is_deleted
        FROM auth_accounts a
        JOIN users u ON u.user_id = a.slack_user_id
        WHERE a.email_lookup_hash = $1
        LIMIT 1
        "#,
    )
    .bind(email_hash)
    .fetch_optional(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;

    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(AccountRow {
        account_id: row.try_get("account_id").map_err(|e| Error::from(e.to_string()))?,
        slack_user_id: row
            .try_get("slack_user_id")
            .map_err(|e| Error::from(e.to_string()))?,
        password_hash: row
            .try_get("password_hash")
            .map_err(|e| Error::from(e.to_string()))?,
        is_anonymous: row
            .try_get("is_anonymous")
            .map_err(|e| Error::from(e.to_string()))?,
        display_name: row
            .try_get("display_name")
            .map_err(|e| Error::from(e.to_string()))?,
        avatar_url: row
            .try_get("avatar_url")
            .map_err(|e| Error::from(e.to_string()))?,
        account_disabled_at: row
            .try_get("account_disabled_at")
            .map_err(|e| Error::from(e.to_string()))?,
        slack_is_active: row
            .try_get("slack_is_active")
            .map_err(|e| Error::from(e.to_string()))?,
        slack_is_deleted: row
            .try_get("slack_is_deleted")
            .map_err(|e| Error::from(e.to_string()))?,
    }))
}

async fn create_session(pool: &PgPool, account_id: Uuid) -> Result<(String, String), Error> {
    let raw_token = generate_token();
    let token_hash = sha256_hex(&raw_token);
    let session_id = Uuid::new_v4();
    let expires_at = Utc::now() + Duration::days(SESSION_TTL_DAYS);
    sqlx::query(
        r#"
        INSERT INTO auth_sessions (id, account_id, session_token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(session_id)
    .bind(account_id)
    .bind(token_hash.clone())
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    Ok((raw_token, token_hash))
}

async fn revoke_all_sessions(pool: &PgPool, account_id: Uuid, reason: &str) -> Result<(), Error> {
    sqlx::query(
        r#"
        UPDATE auth_sessions
        SET revoked_at = NOW(),
            revoke_reason = $2
        WHERE account_id = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(account_id)
    .bind(reason)
    .execute(pool)
    .await
    .map_err(|e| Error::from(e.to_string()))?;
    Ok(())
}

async fn revoke_other_sessions(
    pool: &PgPool,
    account_id: Uuid,
    current_token: Option<String>,
    reason: &str,
) -> Result<(), Error> {
    let current_hash = current_token.map(|token| sha256_hex(&token));
    if let Some(current_hash) = current_hash {
        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET revoked_at = NOW(),
                revoke_reason = $3
            WHERE account_id = $1
              AND session_token_hash <> $2
              AND revoked_at IS NULL
            "#,
        )
        .bind(account_id)
        .bind(current_hash)
        .bind(reason)
        .execute(pool)
        .await
        .map_err(|e| Error::from(e.to_string()))?;
    } else {
        revoke_all_sessions(pool, account_id, reason).await?;
    }
    Ok(())
}

fn ensure_internal_secret(req: &http::Request<Bytes>) -> Option<Response<Bytes>> {
    let expected_secret = env::var("FRONTEND_BACKEND_SHARED_SECRET").unwrap_or_default();
    if expected_secret.is_empty() {
        return Some(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "missing internal secret configuration",
        ));
    }
    let provided = req
        .headers()
        .get(INTERNAL_SECRET_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if provided != expected_secret {
        return Some(error_response(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    None
}

fn session_token_from_headers(req: &http::Request<Bytes>) -> Option<String> {
    req.headers()
        .get(SESSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn parse_json<T: serde::de::DeserializeOwned>(bytes: &Bytes) -> Result<T, Error> {
    serde_json::from_slice(bytes).map_err(|_| Error::from("invalid json body"))
}

fn validate_password_policy(password: &str) -> Option<Response<Bytes>> {
    if password.len() < 12 {
        return Some(error_response(
            StatusCode::BAD_REQUEST,
            "password must be at least 12 characters long",
        ));
    }
    None
}

fn should_expose_reset_token_for_dev() -> bool {
    env::var("AUTH_DEV_EXPOSE_RESET_TOKEN")
        .map(|value| {
            let normalized = value.trim().to_ascii_lowercase();
            matches!(normalized.as_str(), "1" | "true" | "yes" | "on")
        })
        .unwrap_or(false)
}

fn account_profile(display_name: &str, avatar_url: &str) -> (String, String) {
    let next_name = if display_name.trim().is_empty() {
        "User".to_owned()
    } else {
        display_name.to_owned()
    };
    let next_avatar = if avatar_url.trim().is_empty() {
        "/placeholder-user.jpg".to_owned()
    } else {
        avatar_url.to_owned()
    };
    (next_name, next_avatar)
}

fn json_ok<T: Serialize>(value: &T) -> Result<Response<Bytes>, Error> {
    let body = serde_json::to_vec(value).map_err(|e| Error::from(e.to_string()))?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(Bytes::from(body))?)
}

fn error_response(status: StatusCode, message: &str) -> Response<Bytes> {
    let payload = serde_json::json!({
        "ok": false,
        "error": message
    });
    let body = serde_json::to_vec(&payload).unwrap_or_else(|_| b"{\"ok\":false}".to_vec());
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(Bytes::from(body))
        .unwrap_or_else(|_| Response::new(Bytes::from_static(br#"{"ok":false}"#)))
}

fn internal_error_response() -> Result<Response<ResponseBody>, Error> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header("Content-Type", "application/json")
        .body(ResponseBody::from(Bytes::from_static(
            br#"{"ok":false,"error":"internal server error"}"#,
        )))?)
}
