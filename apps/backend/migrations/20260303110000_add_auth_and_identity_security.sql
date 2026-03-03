-- Add encrypted identity fields and Slack status flags on cached users
ALTER TABLE users
    ADD COLUMN email_ciphertext TEXT,
    ADD COLUMN email_lookup_hash TEXT,
    ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN is_deleted BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN disabled_at TIMESTAMPTZ,
    ADD COLUMN last_synced_at TIMESTAMPTZ;

CREATE INDEX idx_users_email_lookup_hash ON users (email_lookup_hash);
CREATE INDEX idx_users_status ON users (is_active, is_deleted);

-- Account table (1 account per Slack user)
CREATE TABLE auth_accounts (
    id UUID PRIMARY KEY,
    slack_user_id TEXT NOT NULL UNIQUE REFERENCES users(user_id),
    email_ciphertext TEXT NOT NULL,
    email_lookup_hash TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    is_anonymous BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disabled_at TIMESTAMPTZ,
    disabled_reason TEXT
);

CREATE INDEX idx_auth_accounts_slack_user_id ON auth_accounts (slack_user_id);

-- Stateful browser sessions
CREATE TABLE auth_sessions (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES auth_accounts(id) ON DELETE CASCADE,
    session_token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    revoke_reason TEXT,
    last_seen_at TIMESTAMPTZ
);

CREATE INDEX idx_auth_sessions_account_state
    ON auth_sessions (account_id, revoked_at, expires_at);

-- Password reset one-time tokens
CREATE TABLE password_reset_tokens (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES auth_accounts(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    used_at TIMESTAMPTZ
);

CREATE INDEX idx_password_reset_tokens_account_id
    ON password_reset_tokens (account_id);
