use std::str::FromStr;

use thiserror::Error;

/// Object provider selected for newly authorized uploads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectStorageProvider {
    /// Cloudflare R2 through its S3-compatible API.
    CloudflareR2,
    /// Amazon Simple Storage Service.
    AmazonS3,
    /// Google Cloud Storage.
    GoogleCloudStorage,
}

impl ObjectStorageProvider {
    /// Returns the stable configuration value for this provider.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CloudflareR2 => "cloudflare-r2",
            Self::AmazonS3 => "aws-s3",
            Self::GoogleCloudStorage => "google-cloud-storage",
        }
    }
}

impl FromStr for ObjectStorageProvider {
    type Err = ObjectStorageProviderParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "cloudflare-r2" => Ok(Self::CloudflareR2),
            "aws-s3" => Ok(Self::AmazonS3),
            "google-cloud-storage" => Ok(Self::GoogleCloudStorage),
            unsupported => Err(ObjectStorageProviderParseError(unsupported.to_owned())),
        }
    }
}

/// Error returned for an unknown provider configuration value.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("unsupported object storage provider: {0}")]
pub struct ObjectStorageProviderParseError(String);

/// Provider selection and non-secret object namespace configuration.
///
/// Credentials are deliberately absent. Concrete SDK adapters will use
/// workload identity or their standard credential chain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectStorageConfig {
    /// Provider used only for new uploads; existing objects retain their own
    /// persisted provider locator.
    pub provider: ObjectStorageProvider,
    /// Private bucket or equivalent object namespace.
    pub bucket: String,
    /// Optional provider/emulator endpoint. R2 requires an explicit endpoint.
    pub endpoint: Option<String>,
    /// Optional region override; otherwise the SDK credential/config chain may
    /// resolve it.
    pub region: Option<String>,
}

impl ObjectStorageConfig {
    /// Creates a validated, non-secret storage configuration.
    pub fn new(
        provider: ObjectStorageProvider,
        bucket: impl Into<String>,
        endpoint: Option<String>,
        region: Option<String>,
    ) -> Result<Self, ObjectStorageConfigError> {
        let bucket = bucket.into();
        if bucket.trim().is_empty() {
            return Err(ObjectStorageConfigError::EmptyBucket);
        }
        if provider == ObjectStorageProvider::CloudflareR2
            && endpoint.as_deref().is_none_or(str::is_empty)
        {
            return Err(ObjectStorageConfigError::MissingR2Endpoint);
        }

        Ok(Self {
            provider,
            bucket,
            endpoint,
            region,
        })
    }
}

/// Validation failures for non-secret object storage configuration.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum ObjectStorageConfigError {
    /// A bucket/object namespace is mandatory for every provider.
    #[error("object storage bucket must not be empty")]
    EmptyBucket,
    /// R2's account-specific S3 endpoint cannot be inferred from a region.
    #[error("Cloudflare R2 requires an explicit S3 API endpoint")]
    MissingR2Endpoint,
}

#[cfg(test)]
mod tests {
    use super::{ObjectStorageConfig, ObjectStorageConfigError, ObjectStorageProvider};

    #[test]
    fn accepts_each_stable_provider_value() {
        for (value, expected) in [
            ("cloudflare-r2", ObjectStorageProvider::CloudflareR2),
            ("aws-s3", ObjectStorageProvider::AmazonS3),
            (
                "google-cloud-storage",
                ObjectStorageProvider::GoogleCloudStorage,
            ),
        ] {
            assert_eq!(value.parse::<ObjectStorageProvider>(), Ok(expected));
        }
    }

    #[test]
    fn requires_an_r2_endpoint() {
        let config = ObjectStorageConfig::new(
            ObjectStorageProvider::CloudflareR2,
            "private-evidence",
            None,
            None,
        );

        assert_eq!(config, Err(ObjectStorageConfigError::MissingR2Endpoint));
    }
}
