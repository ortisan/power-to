use uuid::Uuid;

/// Internal account identifier resolved from an authenticated identity.
///
/// This value is deliberately distinct from an OpenID Connect subject. The
/// identity adapter will own that mapping when OIDC is implemented.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AccountId(Uuid);

impl AccountId {
    /// Wraps an identifier resolved at a trusted boundary.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for outer adapters.
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}
