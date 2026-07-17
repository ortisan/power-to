-- atlas:txmode file

CREATE TABLE private.accounts (
    account_id uuid PRIMARY KEY,
    status text NOT NULL DEFAULT 'active',
    version bigint NOT NULL DEFAULT 1,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),

    CONSTRAINT private_accounts_status_chk CHECK (
        status IN ('active', 'suspended', 'closed')
    ),
    CONSTRAINT private_accounts_version_chk CHECK (version > 0),
    CONSTRAINT private_accounts_timestamps_chk CHECK (updated_at >= created_at)
);

COMMENT ON TABLE private.accounts IS
    'Local accounts separated from provider identities and public civic data.';

CREATE TABLE private.account_identities (
    issuer text NOT NULL,
    subject text NOT NULL,
    account_id uuid NOT NULL
        REFERENCES private.accounts (account_id) ON DELETE RESTRICT,
    linked_at timestamptz NOT NULL DEFAULT transaction_timestamp(),

    PRIMARY KEY (issuer, subject),
    CONSTRAINT private_account_identities_issuer_chk CHECK (
        issuer = btrim(issuer)
        AND char_length(issuer) BETWEEN 1 AND 2048
        AND octet_length(issuer) <= 2048
    ),
    CONSTRAINT private_account_identities_subject_chk CHECK (
        subject = btrim(subject)
        AND char_length(subject) BETWEEN 1 AND 255
        AND octet_length(subject) <= 255
    )
);

COMMENT ON TABLE private.account_identities IS
    'Minimal OIDC issuer/subject mapping; tokens, names, and email addresses are never stored.';

CREATE INDEX private_account_identities_account_idx
    ON private.account_identities (account_id);
