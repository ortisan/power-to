-- atlas:txmode file

-- Bootstrap database capabilities and bounded contexts only. Product tables
-- belong in later, independently reviewed migrations.
CREATE EXTENSION postgis WITH SCHEMA public;

CREATE SCHEMA civic;
COMMENT ON SCHEMA civic IS
    'Public-interest workflows such as reports, moderation, and voting.';

CREATE SCHEMA private;
COMMENT ON SCHEMA private IS
    'Restricted identity, consent, and other sensitive records.';

CREATE SCHEMA audit;
COMMENT ON SCHEMA audit IS
    'Append-oriented audit history and integrity evidence.';

CREATE SCHEMA jobs;
COMMENT ON SCHEMA jobs IS
    'Transactional outbox and background-job coordination.';

CREATE SCHEMA evidence;
COMMENT ON SCHEMA evidence IS
    'Metadata for media and sensor evidence; object bytes remain external.';
