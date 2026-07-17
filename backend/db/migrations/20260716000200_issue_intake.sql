-- atlas:txmode file

-- The database computes audit-event hashes from versioned canonical bytes.
-- The application still computes request fingerprints before persistence.
CREATE EXTENSION pgcrypto WITH SCHEMA public;

CREATE TABLE civic.issues (
    issue_id uuid PRIMARY KEY,
    public_reference uuid NOT NULL UNIQUE,
    version bigint NOT NULL DEFAULT 1,
    status text NOT NULL DEFAULT 'submitted',
    category_key text NOT NULL,
    submission_policy_version text NOT NULL,
    title text NOT NULL,
    summary text NOT NULL,
    problem_statement text NOT NULL,
    affected_community text NOT NULL,
    desired_outcome text NOT NULL,
    public_location public.geography(Point, 4326) NOT NULL,
    public_location_method text NOT NULL,
    location_label text,
    submitted_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    updated_at timestamptz NOT NULL DEFAULT transaction_timestamp(),

    CONSTRAINT civic_issues_version_chk CHECK (version > 0),
    CONSTRAINT civic_issues_status_chk CHECK (
        status IN (
            'submitted',
            'in_moderation',
            'needs_clarification',
            'published',
            'rejected'
        )
    ),
    CONSTRAINT civic_issues_category_key_chk CHECK (
        category_key = btrim(category_key)
        AND char_length(category_key) BETWEEN 1 AND 64
        AND category_key ~ '^[a-z0-9]+(-[a-z0-9]+)*$'
    ),
    CONSTRAINT civic_issues_policy_version_chk CHECK (
        submission_policy_version = btrim(submission_policy_version)
        AND char_length(submission_policy_version) BETWEEN 1 AND 64
    ),
    CONSTRAINT civic_issues_title_chk CHECK (
        title = btrim(title) AND char_length(title) BETWEEN 1 AND 120
    ),
    CONSTRAINT civic_issues_summary_chk CHECK (
        summary = btrim(summary) AND char_length(summary) BETWEEN 1 AND 500
    ),
    CONSTRAINT civic_issues_problem_statement_chk CHECK (
        problem_statement = btrim(problem_statement)
        AND char_length(problem_statement) BETWEEN 1 AND 10000
    ),
    CONSTRAINT civic_issues_affected_community_chk CHECK (
        affected_community = btrim(affected_community)
        AND char_length(affected_community) BETWEEN 1 AND 2000
    ),
    CONSTRAINT civic_issues_desired_outcome_chk CHECK (
        desired_outcome = btrim(desired_outcome)
        AND char_length(desired_outcome) BETWEEN 1 AND 2000
    ),
    CONSTRAINT civic_issues_public_location_method_chk CHECK (
        public_location_method = 'exact-civic-problem-point-v1'
    ),
    CONSTRAINT civic_issues_public_location_chk CHECK (
        NOT public.ST_IsEmpty(public_location::public.geometry)
        AND public.ST_IsValid(public_location::public.geometry)
    ),
    CONSTRAINT civic_issues_location_label_chk CHECK (
        location_label IS NULL
        OR (
            location_label = btrim(location_label)
            AND char_length(location_label) BETWEEN 1 AND 200
        )
    ),
    CONSTRAINT civic_issues_timestamps_chk CHECK (updated_at >= submitted_at)
);

COMMENT ON COLUMN civic.issues.public_reference IS
    'Random UUIDv4 exposed by HTTP; never use the time-ordered internal ID publicly.';
COMMENT ON COLUMN civic.issues.public_location IS
    'Confirmed civic problem point. It is not device history, residence evidence, or voting eligibility.';

CREATE INDEX civic_issues_moderation_queue_idx
    ON civic.issues (status, submitted_at, issue_id);
CREATE INDEX civic_issues_public_location_gix
    ON civic.issues USING gist (public_location);

CREATE TABLE private.issue_submission_context (
    issue_id uuid PRIMARY KEY
        REFERENCES civic.issues (issue_id) ON DELETE RESTRICT,
    submitted_by uuid NOT NULL,
    observed_location public.geography(Point, 4326) NOT NULL,
    geometry_source text NOT NULL,
    public_attribution_consent boolean NOT NULL DEFAULT false,
    privacy_notice_version text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),

    CONSTRAINT private_issue_context_geometry_source_chk CHECK (
        geometry_source IN ('map_selection', 'device_location', 'geocoded_search')
    ),
    CONSTRAINT private_issue_context_observed_location_chk CHECK (
        NOT public.ST_IsEmpty(observed_location::public.geometry)
        AND public.ST_IsValid(observed_location::public.geometry)
    ),
    CONSTRAINT private_issue_context_privacy_version_chk CHECK (
        privacy_notice_version = btrim(privacy_notice_version)
        AND char_length(privacy_notice_version) BETWEEN 1 AND 64
    ),
    CONSTRAINT private_issue_context_actor_uk UNIQUE (issue_id, submitted_by)
);

COMMENT ON TABLE private.issue_submission_context IS
    'Restricted submitter, consent, provenance, and confirmed-location context.';

CREATE INDEX private_issue_context_observed_location_gix
    ON private.issue_submission_context USING gist (observed_location);
CREATE INDEX private_issue_context_submitted_by_idx
    ON private.issue_submission_context (submitted_by, issue_id);

CREATE TABLE private.issue_submission_idempotency (
    submitted_by uuid NOT NULL,
    operation_version smallint NOT NULL DEFAULT 1,
    idempotency_key_hash bytea NOT NULL,
    request_fingerprint bytea NOT NULL,
    fingerprint_version smallint NOT NULL DEFAULT 1,
    issue_id uuid NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),

    PRIMARY KEY (submitted_by, operation_version, idempotency_key_hash),
    CONSTRAINT private_issue_idempotency_operation_version_chk CHECK (
        operation_version > 0
    ),
    CONSTRAINT private_issue_idempotency_key_hash_chk CHECK (
        octet_length(idempotency_key_hash) = 32
    ),
    CONSTRAINT private_issue_request_fingerprint_chk CHECK (
        fingerprint_version > 0 AND octet_length(request_fingerprint) = 32
    ),
    CONSTRAINT private_issue_idempotency_context_fk
        FOREIGN KEY (issue_id, submitted_by)
        REFERENCES private.issue_submission_context (issue_id, submitted_by)
        ON DELETE RESTRICT
        DEFERRABLE INITIALLY DEFERRED
);

COMMENT ON TABLE private.issue_submission_idempotency IS
    'Stores only SHA-256 key digests and versioned normalized-request fingerprints.';

CREATE TABLE audit.events (
    event_id uuid PRIMARY KEY,
    stream_type text NOT NULL,
    stream_id uuid NOT NULL,
    stream_version bigint NOT NULL,
    event_type text NOT NULL,
    actor_id uuid NOT NULL,
    canonical_format smallint NOT NULL DEFAULT 1,
    canonical_event bytea NOT NULL,
    occurred_at timestamptz NOT NULL,
    previous_stream_version bigint,
    previous_hash bytea,
    event_hash bytea GENERATED ALWAYS AS (
        public.digest(
            COALESCE(previous_hash, decode(repeat('00', 32), 'hex'))
                || canonical_event,
            'sha256'
        )
    ) STORED,

    CONSTRAINT audit_events_stream_type_chk CHECK (
        stream_type ~ '^[a-z][a-z0-9._-]{2,63}$'
    ),
    CONSTRAINT audit_events_event_type_chk CHECK (
        event_type ~ '^[a-z][a-z0-9._-]{2,127}$'
    ),
    CONSTRAINT audit_events_stream_version_chk CHECK (stream_version > 0),
    CONSTRAINT audit_events_canonical_event_chk CHECK (
        canonical_format > 0
        AND octet_length(canonical_event) BETWEEN 1 AND 65536
    ),
    CONSTRAINT audit_events_chain_shape_chk CHECK (
        (
            stream_version = 1
            AND previous_stream_version IS NULL
            AND previous_hash IS NULL
        )
        OR
        (
            stream_version > 1
            AND previous_stream_version = stream_version - 1
            AND octet_length(previous_hash) = 32
        )
    ),
    CONSTRAINT audit_events_stream_version_uk
        UNIQUE (stream_type, stream_id, stream_version),
    CONSTRAINT audit_events_chain_target_uk
        UNIQUE (stream_type, stream_id, stream_version, event_hash),
    CONSTRAINT audit_events_previous_event_fk
        FOREIGN KEY (
            stream_type,
            stream_id,
            previous_stream_version,
            previous_hash
        )
        REFERENCES audit.events (
            stream_type,
            stream_id,
            stream_version,
            event_hash
        )
);

COMMENT ON COLUMN audit.events.canonical_event IS
    'Versioned canonical binary envelope; never PostgreSQL jsonb::text.';
COMMENT ON COLUMN audit.events.event_hash IS
    'SHA-256(previous_hash || canonical_event), generated by PostgreSQL.';

CREATE INDEX audit_events_occurred_at_brin
    ON audit.events USING brin (occurred_at);

CREATE FUNCTION audit.reject_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'audit events are append-only'
        USING ERRCODE = '55000';
END;
$$;

CREATE TRIGGER audit_events_append_only
BEFORE UPDATE OR DELETE ON audit.events
FOR EACH ROW EXECUTE FUNCTION audit.reject_event_mutation();

CREATE TRIGGER audit_events_reject_truncate
BEFORE TRUNCATE ON audit.events
FOR EACH STATEMENT EXECUTE FUNCTION audit.reject_event_mutation();

CREATE TABLE jobs.outbox_messages (
    message_id uuid PRIMARY KEY,
    audit_event_id uuid NOT NULL UNIQUE
        REFERENCES audit.events (event_id) ON DELETE RESTRICT,
    topic text NOT NULL,
    payload_version smallint NOT NULL,
    payload jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    available_at timestamptz NOT NULL DEFAULT transaction_timestamp(),
    attempt_count integer NOT NULL DEFAULT 0,
    lease_owner text,
    lease_until timestamptz,
    completed_at timestamptz,
    last_error_code text,

    CONSTRAINT jobs_outbox_topic_chk CHECK (
        topic ~ '^[a-z][a-z0-9._-]{2,127}$'
    ),
    CONSTRAINT jobs_outbox_payload_chk CHECK (
        payload_version > 0 AND jsonb_typeof(payload) = 'object'
    ),
    CONSTRAINT jobs_outbox_attempt_count_chk CHECK (attempt_count >= 0),
    CONSTRAINT jobs_outbox_lease_chk CHECK (
        (lease_owner IS NULL) = (lease_until IS NULL)
    ),
    CONSTRAINT jobs_outbox_completed_at_chk CHECK (
        completed_at IS NULL OR completed_at >= created_at
    )
);

COMMENT ON TABLE jobs.outbox_messages IS
    'Minimal privacy-safe integration events written with domain state and audit evidence.';

CREATE INDEX jobs_outbox_ready_idx
    ON jobs.outbox_messages (available_at, created_at, message_id)
    WHERE completed_at IS NULL;
