use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{AlgorithmParameters, Jwk, JwkSet, KeyAlgorithm, KeyOperations, PublicKeyUse},
};
use powerto_application::identity::{
    AccountDirectory, AccountDirectoryError, ActorAuthenticator, AuthenticatedActor,
    AuthenticationError, ExternalIdentity, PresentedCredential,
};
use reqwest::{Client, Response, Url, redirect::Policy};
use serde::{Deserialize, de::DeserializeOwned};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

const MAX_TOKEN_LENGTH: usize = 16 * 1024;
const MAX_KEY_ID_LENGTH: usize = 128;
const MAX_DISCOVERY_BYTES: usize = 64 * 1024;
const MAX_JWKS_BYTES: usize = 1024 * 1024;
const UNKNOWN_KEY_REFRESH_COOLDOWN: Duration = Duration::from_secs(30);
const MAXIMUM_STALE_JWKS: Duration = Duration::from_secs(60 * 60);

/// Security and availability settings for an OIDC resource server.
#[derive(Clone)]
pub struct OidcConfig {
    /// Exact issuer accepted in access tokens and discovery metadata.
    pub issuer: String,
    /// API audience required in access tokens.
    pub audience: String,
    /// Allowed clock difference for `exp`, `nbf`, and `iat` checks.
    pub clock_skew: Duration,
    /// Total and connection timeout for discovery and JWKS requests.
    pub http_timeout: Duration,
    /// Time before the cached JWKS is refreshed.
    pub jwks_refresh_interval: Duration,
    /// Allows plain HTTP only when both issuer and JWKS hosts are loopback.
    pub allow_insecure_loopback_http: bool,
}

/// Opaque startup failure that cannot expose provider metadata or URLs.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("OIDC authentication could not be initialized")]
pub struct OidcInitializationError;

/// OIDC JWT access-token authenticator backed by discovery and cached JWKS.
pub struct OidcActorAuthenticator {
    issuer: String,
    audience: String,
    clock_skew_seconds: u64,
    jwks_refresh_interval: Duration,
    client: Client,
    jwks_uri: Url,
    state: RwLock<JwksState>,
    refresh_lock: Mutex<()>,
    directory: Arc<dyn AccountDirectory>,
}

impl OidcActorAuthenticator {
    /// Discovers the provider and retrieves its signing keys before serving.
    pub async fn discover(
        config: OidcConfig,
        directory: Arc<dyn AccountDirectory>,
    ) -> Result<Self, OidcInitializationError> {
        validate_bounded_value(&config.audience, 255)?;
        validate_bounded_value(&config.issuer, 2_048)?;
        let issuer_url =
            validate_provider_url(&config.issuer, config.allow_insecure_loopback_http)?;
        if issuer_url.query().is_some() || issuer_url.fragment().is_some() {
            return Err(OidcInitializationError);
        }

        let client = Client::builder()
            .connect_timeout(config.http_timeout)
            .timeout(config.http_timeout)
            .redirect(Policy::none())
            .no_proxy()
            .user_agent(concat!("powerto-api/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|_| OidcInitializationError)?;
        let discovery_url = Url::parse(&format!(
            "{}/.well-known/openid-configuration",
            config.issuer.trim_end_matches('/')
        ))
        .map_err(|_| OidcInitializationError)?;
        let discovery: DiscoveryDocument = fetch_json(
            &client,
            discovery_url,
            MAX_DISCOVERY_BYTES,
            "oidc.discovery",
        )
        .await?;
        if discovery.issuer != config.issuer {
            tracing::warn!(operation = "oidc.discovery", reason = "issuer_mismatch");
            return Err(OidcInitializationError);
        }
        let jwks_uri =
            validate_provider_url(&discovery.jwks_uri, config.allow_insecure_loopback_http)?;
        let keys: JwkSet =
            fetch_json(&client, jwks_uri.clone(), MAX_JWKS_BYTES, "oidc.jwks").await?;
        validate_jwks(&keys)?;

        Ok(Self {
            issuer: config.issuer,
            audience: config.audience,
            clock_skew_seconds: config.clock_skew.as_secs(),
            jwks_refresh_interval: config.jwks_refresh_interval,
            client,
            jwks_uri,
            state: RwLock::new(JwksState {
                keys,
                fetched_at: Instant::now(),
                last_unknown_key_refresh: None,
            }),
            refresh_lock: Mutex::new(()),
            directory,
        })
    }

    async fn authenticate_token(
        &self,
        token: &str,
    ) -> Result<AuthenticatedActor, AuthenticationError> {
        if token.is_empty() || token.len() > MAX_TOKEN_LENGTH || token.split('.').count() != 3 {
            return Err(AuthenticationError::InvalidCredential);
        }

        let header = decode_header(token).map_err(|_| AuthenticationError::InvalidCredential)?;
        if header.typ.as_deref() != Some("at+jwt")
            && header.typ.as_deref() != Some("application/at+jwt")
        {
            return Err(AuthenticationError::InvalidCredential);
        }
        if header.alg != Algorithm::RS256
            || header.jku.is_some()
            || header.jwk.is_some()
            || header.x5u.is_some()
            || header.crit.is_some()
        {
            return Err(AuthenticationError::InvalidCredential);
        }
        let kid = header
            .kid
            .as_deref()
            .filter(|value| !value.is_empty() && value.len() <= MAX_KEY_ID_LENGTH)
            .ok_or(AuthenticationError::InvalidCredential)?;
        let jwk = self.key_for(kid).await?;
        validate_signing_key(&jwk, header.alg)?;
        let key =
            DecodingKey::from_jwk(&jwk).map_err(|_| AuthenticationError::InvalidCredential)?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.leeway = self.clock_skew_seconds;
        validation.validate_nbf = true;
        validation.set_audience(&[self.audience.as_str()]);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let token_data = decode::<AccessTokenClaims>(token, &key, &validation)
            .map_err(|_| AuthenticationError::InvalidCredential)?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AuthenticationError::Unavailable)?
            .as_secs();
        if token_data.claims.iat > token_data.claims.exp
            || token_data.claims.iat > now.saturating_add(self.clock_skew_seconds)
        {
            return Err(AuthenticationError::InvalidCredential);
        }
        let identity = ExternalIdentity::new(token_data.claims.iss, token_data.claims.sub)
            .map_err(|_| AuthenticationError::InvalidCredential)?;
        let account_id = self
            .directory
            .resolve_or_provision(&identity)
            .await
            .map_err(map_directory_error)?;
        Ok(AuthenticatedActor::new(account_id))
    }

    async fn key_for(&self, kid: &str) -> Result<Jwk, AuthenticationError> {
        let (cached, fresh, cooldown_active) = {
            let state = self.state.read().await;
            (
                state.keys.find(kid).cloned(),
                state.fetched_at.elapsed() < self.jwks_refresh_interval,
                state
                    .last_unknown_key_refresh
                    .is_some_and(|instant| instant.elapsed() < UNKNOWN_KEY_REFRESH_COOLDOWN),
            )
        };
        if cached.is_some() && fresh {
            return cached.ok_or(AuthenticationError::InvalidCredential);
        }
        if cached.is_none() && cooldown_active {
            return Err(AuthenticationError::InvalidCredential);
        }

        let _refresh_guard = self.refresh_lock.lock().await;
        let (cached_after_wait, should_refresh, cached_usable_on_failure) = {
            let state = self.state.read().await;
            let cached_after_wait = state.keys.find(kid).cloned();
            let fresh_after_wait = state.fetched_at.elapsed() < self.jwks_refresh_interval;
            let cooldown_after_wait = state
                .last_unknown_key_refresh
                .is_some_and(|instant| instant.elapsed() < UNKNOWN_KEY_REFRESH_COOLDOWN);
            let should_refresh = if cached_after_wait.is_some() {
                !fresh_after_wait
            } else {
                !cooldown_after_wait
            };
            let cached_usable_on_failure = state.fetched_at.elapsed()
                <= self
                    .jwks_refresh_interval
                    .saturating_add(MAXIMUM_STALE_JWKS);
            (cached_after_wait, should_refresh, cached_usable_on_failure)
        };
        if !should_refresh {
            return cached_after_wait.ok_or(AuthenticationError::InvalidCredential);
        }

        match self.refresh_keys(cached_after_wait.is_none()).await {
            Ok(()) => self
                .state
                .read()
                .await
                .keys
                .find(kid)
                .cloned()
                .ok_or(AuthenticationError::InvalidCredential),
            Err(_) if cached_usable_on_failure => {
                cached_after_wait.ok_or(AuthenticationError::Unavailable)
            }
            Err(_) => Err(AuthenticationError::Unavailable),
        }
    }

    async fn refresh_keys(&self, unknown_key: bool) -> Result<(), OidcInitializationError> {
        let keys: JwkSet = fetch_json(
            &self.client,
            self.jwks_uri.clone(),
            MAX_JWKS_BYTES,
            "oidc.jwks.refresh",
        )
        .await?;
        validate_jwks(&keys)?;
        let mut state = self.state.write().await;
        state.keys = keys;
        state.fetched_at = Instant::now();
        if unknown_key {
            state.last_unknown_key_refresh = Some(Instant::now());
        }
        Ok(())
    }
}

#[async_trait]
impl ActorAuthenticator for OidcActorAuthenticator {
    async fn authenticate(
        &self,
        credential: &PresentedCredential,
    ) -> Result<AuthenticatedActor, AuthenticationError> {
        let result = self.authenticate_token(credential.expose()).await;
        let outcome = match result {
            Ok(_) => "success",
            Err(AuthenticationError::InvalidCredential) => "invalid_credential",
            Err(AuthenticationError::Forbidden) => "forbidden",
            Err(AuthenticationError::Unavailable) => "unavailable",
        };
        tracing::info!(
            operation = "oidc.authenticate",
            outcome,
            "authentication completed"
        );
        result
    }
}

struct JwksState {
    keys: JwkSet,
    fetched_at: Instant,
    last_unknown_key_refresh: Option<Instant>,
}

#[derive(Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    jwks_uri: String,
}

#[derive(Deserialize)]
struct AccessTokenClaims {
    iss: String,
    sub: String,
    exp: u64,
    iat: u64,
}

async fn fetch_json<T: DeserializeOwned>(
    client: &Client,
    url: Url,
    maximum_bytes: usize,
    operation: &'static str,
) -> Result<T, OidcInitializationError> {
    let response = client.get(url).send().await.map_err(|_| {
        tracing::warn!(operation, reason = "request_failed");
        OidcInitializationError
    })?;
    if !response.status().is_success() {
        tracing::warn!(operation, reason = "unexpected_status");
        return Err(OidcInitializationError);
    }
    let bytes = bounded_body(response, maximum_bytes, operation).await?;
    serde_json::from_slice(&bytes).map_err(|_| {
        tracing::warn!(operation, reason = "invalid_json");
        OidcInitializationError
    })
}

async fn bounded_body(
    mut response: Response,
    maximum_bytes: usize,
    operation: &'static str,
) -> Result<Vec<u8>, OidcInitializationError> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum_bytes as u64)
    {
        tracing::warn!(operation, reason = "response_too_large");
        return Err(OidcInitializationError);
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| OidcInitializationError)?
    {
        if body.len().saturating_add(chunk.len()) > maximum_bytes {
            tracing::warn!(operation, reason = "response_too_large");
            return Err(OidcInitializationError);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

fn validate_provider_url(
    value: &str,
    allow_insecure_loopback_http: bool,
) -> Result<Url, OidcInitializationError> {
    let url = Url::parse(value).map_err(|_| OidcInitializationError)?;
    if url.username() != "" || url.password().is_some() || url.host_str().is_none() {
        return Err(OidcInitializationError);
    }
    match url.scheme() {
        "https" => Ok(url),
        "http" if allow_insecure_loopback_http && is_loopback_host(&url) => Ok(url),
        _ => Err(OidcInitializationError),
    }
}

fn is_loopback_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn validate_bounded_value(
    value: &str,
    maximum_length: usize,
) -> Result<(), OidcInitializationError> {
    if value.trim().is_empty() || value.trim() != value || value.len() > maximum_length {
        Err(OidcInitializationError)
    } else {
        Ok(())
    }
}

fn validate_jwks(keys: &JwkSet) -> Result<(), OidcInitializationError> {
    if keys.keys.is_empty() || keys.keys.len() > 100 {
        return Err(OidcInitializationError);
    }
    let mut key_ids = std::collections::HashSet::new();
    for key in &keys.keys {
        let Some(key_id) = key.common.key_id.as_deref() else {
            continue;
        };
        if key_id.is_empty() || key_id.len() > MAX_KEY_ID_LENGTH || !key_ids.insert(key_id) {
            return Err(OidcInitializationError);
        }
    }
    Ok(())
}

fn validate_signing_key(jwk: &Jwk, algorithm: Algorithm) -> Result<(), AuthenticationError> {
    if algorithm != Algorithm::RS256
        || !matches!(jwk.algorithm, AlgorithmParameters::RSA(_))
        || jwk.common.key_id.as_deref().is_none_or(str::is_empty)
        || jwk.common.key_algorithm != Some(KeyAlgorithm::RS256)
        || matches!(jwk.common.public_key_use, Some(ref usage) if *usage != PublicKeyUse::Signature)
        || jwk
            .common
            .key_operations
            .as_ref()
            .is_some_and(|operations| !operations.contains(&KeyOperations::Verify))
    {
        return Err(AuthenticationError::InvalidCredential);
    }
    Ok(())
}

const fn map_directory_error(error: AccountDirectoryError) -> AuthenticationError {
    match error {
        AccountDirectoryError::Forbidden => AuthenticationError::Forbidden,
        AccountDirectoryError::Unavailable | AccountDirectoryError::InvalidStoredData => {
            AuthenticationError::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use async_trait::async_trait;
    use jsonwebtoken::{Algorithm, EncodingKey, Header, encode, jwk::Jwk};
    use powerto_application::identity::{
        AccountDirectory, AccountDirectoryError, AuthenticationError, ExternalIdentity,
    };
    use powerto_domain::AccountId;
    use rand::rngs::OsRng;
    use rsa::{RsaPrivateKey, pkcs1::EncodeRsaPrivateKey as _};
    use serde::Serialize;
    use tokio::sync::{Mutex, RwLock};

    use super::{
        JwksState, OidcActorAuthenticator, OidcInitializationError, is_loopback_host,
        validate_provider_url,
    };

    #[test]
    fn insecure_provider_urls_are_loopback_only() {
        assert!(validate_provider_url("http://127.0.0.1:8080/realms/local", true).is_ok());
        assert!(validate_provider_url("http://localhost:8080/realms/local", true).is_ok());
        assert_eq!(
            validate_provider_url("http://example.com/realms/local", true).err(),
            Some(OidcInitializationError)
        );
        assert!(validate_provider_url("http://127.0.0.1:8080/realms/local", false).is_err());
    }

    #[test]
    fn loopback_detection_rejects_lookalike_hosts() {
        let url = match reqwest::Url::parse("http://localhost.example.test") {
            Ok(url) => url,
            Err(error) => panic!("test URL should parse: {error}"),
        };
        assert!(!is_loopback_host(&url));
    }

    #[tokio::test]
    async fn verifies_a_strict_signed_access_token_and_audience() {
        let private_key = match RsaPrivateKey::new(&mut OsRng, 2_048) {
            Ok(key) => key,
            Err(error) => panic!("test RSA key generation failed: {error}"),
        };
        let der = match private_key.to_pkcs1_der() {
            Ok(der) => der,
            Err(error) => panic!("test RSA key encoding failed: {error}"),
        };
        let encoding_key = EncodingKey::from_rsa_der(der.as_bytes());
        let mut jwk = match Jwk::from_encoding_key(&encoding_key, Algorithm::RS256) {
            Ok(jwk) => jwk,
            Err(error) => panic!("test JWK creation failed: {error}"),
        };
        jwk.common.key_id = Some("test-signing-key".to_owned());
        let account_id = AccountId::new();
        let jwks_uri = match reqwest::Url::parse("https://identity.test/jwks") {
            Ok(url) => url,
            Err(error) => panic!("test JWKS URL should parse: {error}"),
        };
        let authenticator = OidcActorAuthenticator {
            issuer: "https://identity.test/realms/civic".to_owned(),
            audience: "powerto-api".to_owned(),
            clock_skew_seconds: 30,
            jwks_refresh_interval: Duration::from_secs(300),
            client: reqwest::Client::new(),
            jwks_uri,
            state: RwLock::new(JwksState {
                keys: jsonwebtoken::jwk::JwkSet { keys: vec![jwk] },
                fetched_at: std::time::Instant::now(),
                last_unknown_key_refresh: None,
            }),
            refresh_lock: Mutex::new(()),
            directory: Arc::new(FixedDirectory { account_id }),
        };
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(error) => panic!("test clock should follow the Unix epoch: {error}"),
        };
        let valid = signed_token(&encoding_key, "powerto-api", now);
        let actor = match authenticator.authenticate_token(&valid).await {
            Ok(actor) => actor,
            Err(error) => panic!("valid access token was rejected: {error}"),
        };
        assert!(actor.account_id() == account_id);

        let wrong_audience = signed_token(&encoding_key, "another-api", now);
        assert!(matches!(
            authenticator.authenticate_token(&wrong_audience).await,
            Err(AuthenticationError::InvalidCredential)
        ));
    }

    fn signed_token(key: &EncodingKey, audience: &str, now: u64) -> String {
        let mut header = Header::new(Algorithm::RS256);
        header.typ = Some("at+jwt".to_owned());
        header.kid = Some("test-signing-key".to_owned());
        let claims = TestClaims {
            iss: "https://identity.test/realms/civic",
            sub: "test-subject",
            aud: audience,
            exp: now + 300,
            iat: now,
        };
        match encode(&header, &claims, key) {
            Ok(token) => token,
            Err(error) => panic!("test access token signing failed: {error}"),
        }
    }

    #[derive(Serialize)]
    struct TestClaims<'a> {
        iss: &'a str,
        sub: &'a str,
        aud: &'a str,
        exp: u64,
        iat: u64,
    }

    struct FixedDirectory {
        account_id: AccountId,
    }

    #[async_trait]
    impl AccountDirectory for FixedDirectory {
        async fn resolve_or_provision(
            &self,
            _identity: &ExternalIdentity,
        ) -> Result<AccountId, AccountDirectoryError> {
            Ok(self.account_id)
        }
    }
}
