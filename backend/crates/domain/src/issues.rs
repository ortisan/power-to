use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

/// Policy identifier applied to issue submissions created by this release.
pub const CURRENT_SUBMISSION_POLICY: &str = "issue-submission-v1";

const TITLE_MAX_CHARS: usize = 120;
const CATEGORY_MAX_CHARS: usize = 64;
const SUMMARY_MAX_CHARS: usize = 500;
const PROBLEM_MAX_CHARS: usize = 10_000;
const AFFECTED_COMMUNITY_MAX_CHARS: usize = 2_000;
const DESIRED_OUTCOME_MAX_CHARS: usize = 2_000;
const LOCATION_LABEL_MAX_CHARS: usize = 200;
const POLICY_VERSION_MAX_CHARS: usize = 64;

/// Internal identifier for a civic issue.
///
/// New identifiers use UUIDv7 for database locality. They are never exposed by
/// the public HTTP contract.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IssueId(Uuid);

impl IssueId {
    /// Creates a time-ordered internal identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Wraps an identifier loaded from a trusted boundary.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for persistence adapters.
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for IssueId {
    fn default() -> Self {
        Self::new()
    }
}

/// Random external reference for an issue.
///
/// UUIDv4 avoids disclosing the creation timestamp embedded in the internal
/// UUIDv7. This reference is an identifier, not an authorization capability.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct IssueReference(Uuid);

impl IssueReference {
    /// Creates a random external reference.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Wraps a reference loaded from a trusted boundary.
    #[must_use]
    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the UUID used at outer boundaries.
    #[must_use]
    pub const fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for IssueReference {
    fn default() -> Self {
        Self::new()
    }
}

/// Initial moderation lifecycle states understood by this release.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueStatus {
    /// Submitted privately and waiting for moderation.
    Submitted,
    /// A moderator has started reviewing the issue.
    InModeration,
    /// The submitter must provide additional information.
    NeedsClarification,
    /// The issue is approved for public participation.
    Published,
    /// The issue was rejected under a recorded policy and reason.
    Rejected,
}

impl IssueStatus {
    /// Returns the stable persistence and API representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::InModeration => "in_moderation",
            Self::NeedsClarification => "needs_clarification",
            Self::Published => "published",
            Self::Rejected => "rejected",
        }
    }
}

impl FromStr for IssueStatus {
    type Err = IssueStatusParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "submitted" => Ok(Self::Submitted),
            "in_moderation" => Ok(Self::InModeration),
            "needs_clarification" => Ok(Self::NeedsClarification),
            "published" => Ok(Self::Published),
            "rejected" => Ok(Self::Rejected),
            _ => Err(IssueStatusParseError),
        }
    }
}

/// Error returned when persisted issue status is unknown.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("unknown issue status")]
pub struct IssueStatusParseError;

/// How the citizen selected the confirmed problem point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GeometrySource {
    /// The citizen selected or adjusted the point on a map.
    MapSelection,
    /// The app proposed the device's current location before confirmation.
    DeviceLocation,
    /// A text search was geocoded and then confirmed on a map.
    GeocodedSearch,
}

impl GeometrySource {
    /// Returns the stable persistence representation.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MapSelection => "map_selection",
            Self::DeviceLocation => "device_location",
            Self::GeocodedSearch => "geocoded_search",
        }
    }
}

impl FromStr for GeometrySource {
    type Err = GeometrySourceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "map_selection" => Ok(Self::MapSelection),
            "device_location" => Ok(Self::DeviceLocation),
            "geocoded_search" => Ok(Self::GeocodedSearch),
            _ => Err(GeometrySourceParseError),
        }
    }
}

/// Error returned when persisted geometry provenance is unknown.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
#[error("unknown geometry source")]
pub struct GeometrySourceParseError;

/// Confirmed WGS 84 point describing the civic problem, not the user's route
/// or proof of residence.
#[derive(Clone, Copy, PartialEq)]
pub struct GeoPoint {
    longitude: f64,
    latitude: f64,
}

impl GeoPoint {
    /// Validates and creates a WGS 84 point.
    pub fn new(longitude: f64, latitude: f64) -> Result<Self, IssueValidationError> {
        if !longitude.is_finite()
            || !latitude.is_finite()
            || !(-180.0..=180.0).contains(&longitude)
            || !(-90.0..=90.0).contains(&latitude)
        {
            return Err(IssueValidationError::InvalidCoordinate);
        }

        Ok(Self {
            longitude,
            latitude,
        })
    }

    /// Returns longitude in decimal degrees.
    #[must_use]
    pub const fn longitude(self) -> f64 {
        self.longitude
    }

    /// Returns latitude in decimal degrees.
    #[must_use]
    pub const fn latitude(self) -> f64 {
        self.latitude
    }
}

/// Unvalidated values supplied to the domain submission factory.
pub struct IssueSubmissionInput {
    pub title: String,
    pub category_id: String,
    pub summary: String,
    pub problem_statement: String,
    pub affected_community: String,
    pub desired_outcome: String,
    pub point: GeoPoint,
    pub geometry_source: GeometrySource,
    pub location_confirmed: bool,
    pub location_label: Option<String>,
    pub public_attribution: bool,
    pub privacy_notice_version: String,
    pub privacy_notice_accepted: bool,
}

/// Validated issue waiting to be persisted atomically.
#[derive(Clone)]
pub struct IssueSubmission {
    id: IssueId,
    reference: IssueReference,
    title: String,
    category_id: String,
    summary: String,
    problem_statement: String,
    affected_community: String,
    desired_outcome: String,
    point: GeoPoint,
    geometry_source: GeometrySource,
    location_label: Option<String>,
    public_attribution: bool,
    privacy_notice_version: String,
    submission_policy_version: String,
}

impl IssueSubmission {
    /// Validates citizen input and assigns server-owned identifiers.
    pub fn new(input: IssueSubmissionInput) -> Result<Self, IssueValidationError> {
        Self::with_identity_and_policy(
            IssueId::new(),
            IssueReference::new(),
            CURRENT_SUBMISSION_POLICY.to_owned(),
            input,
        )
    }

    fn with_identity_and_policy(
        id: IssueId,
        reference: IssueReference,
        submission_policy_version: String,
        input: IssueSubmissionInput,
    ) -> Result<Self, IssueValidationError> {
        if !input.location_confirmed {
            return Err(IssueValidationError::LocationNotConfirmed);
        }
        if !input.privacy_notice_accepted {
            return Err(IssueValidationError::PrivacyNoticeNotAccepted);
        }

        let title = required_text(input.title, IssueField::Title, TITLE_MAX_CHARS)?;
        let category_id = category_id(input.category_id)?;
        let summary = required_text(input.summary, IssueField::Summary, SUMMARY_MAX_CHARS)?;
        let problem_statement = required_text(
            input.problem_statement,
            IssueField::ProblemStatement,
            PROBLEM_MAX_CHARS,
        )?;
        let affected_community = required_text(
            input.affected_community,
            IssueField::AffectedCommunity,
            AFFECTED_COMMUNITY_MAX_CHARS,
        )?;
        let desired_outcome = required_text(
            input.desired_outcome,
            IssueField::DesiredOutcome,
            DESIRED_OUTCOME_MAX_CHARS,
        )?;
        let privacy_notice_version = required_text(
            input.privacy_notice_version,
            IssueField::PrivacyNoticeVersion,
            POLICY_VERSION_MAX_CHARS,
        )?;
        let submission_policy_version = required_text(
            submission_policy_version,
            IssueField::SubmissionPolicyVersion,
            POLICY_VERSION_MAX_CHARS,
        )?;
        let location_label = optional_text(
            input.location_label,
            IssueField::LocationLabel,
            LOCATION_LABEL_MAX_CHARS,
        )?;

        Ok(Self {
            id,
            reference,
            title,
            category_id,
            summary,
            problem_statement,
            affected_community,
            desired_outcome,
            point: input.point,
            geometry_source: input.geometry_source,
            location_label,
            public_attribution: input.public_attribution,
            privacy_notice_version,
            submission_policy_version,
        })
    }

    #[must_use]
    pub const fn id(&self) -> IssueId {
        self.id
    }

    #[must_use]
    pub const fn reference(&self) -> IssueReference {
        self.reference
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub fn category_id(&self) -> &str {
        &self.category_id
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }

    #[must_use]
    pub fn problem_statement(&self) -> &str {
        &self.problem_statement
    }

    #[must_use]
    pub fn affected_community(&self) -> &str {
        &self.affected_community
    }

    #[must_use]
    pub fn desired_outcome(&self) -> &str {
        &self.desired_outcome
    }

    #[must_use]
    pub const fn point(&self) -> GeoPoint {
        self.point
    }

    #[must_use]
    pub const fn geometry_source(&self) -> GeometrySource {
        self.geometry_source
    }

    #[must_use]
    pub fn location_label(&self) -> Option<&str> {
        self.location_label.as_deref()
    }

    #[must_use]
    pub const fn public_attribution(&self) -> bool {
        self.public_attribution
    }

    #[must_use]
    pub fn privacy_notice_version(&self) -> &str {
        &self.privacy_notice_version
    }

    #[must_use]
    pub fn submission_policy_version(&self) -> &str {
        &self.submission_policy_version
    }
}

/// Persisted issue as visible to its authenticated submitter.
#[derive(Clone)]
pub struct Issue {
    submission: IssueSubmission,
    status: IssueStatus,
    submitted_at: DateTime<Utc>,
    version: u64,
}

impl Issue {
    /// Completes a validated submission using the database transaction time.
    #[must_use]
    pub const fn from_submission(submission: IssueSubmission, submitted_at: DateTime<Utc>) -> Self {
        Self {
            submission,
            status: IssueStatus::Submitted,
            submitted_at,
            version: 1,
        }
    }

    /// Rehydrates and revalidates persisted values at the adapter boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn rehydrate(
        id: IssueId,
        reference: IssueReference,
        title: String,
        category_id: String,
        summary: String,
        problem_statement: String,
        affected_community: String,
        desired_outcome: String,
        point: GeoPoint,
        geometry_source: GeometrySource,
        location_label: Option<String>,
        public_attribution: bool,
        privacy_notice_version: String,
        submission_policy_version: String,
        status: IssueStatus,
        submitted_at: DateTime<Utc>,
        version: u64,
    ) -> Result<Self, IssueValidationError> {
        if version == 0 {
            return Err(IssueValidationError::InvalidVersion);
        }
        let submission = IssueSubmission::with_identity_and_policy(
            id,
            reference,
            submission_policy_version,
            IssueSubmissionInput {
                title,
                category_id,
                summary,
                problem_statement,
                affected_community,
                desired_outcome,
                point,
                geometry_source,
                location_confirmed: true,
                location_label,
                public_attribution,
                privacy_notice_version,
                privacy_notice_accepted: true,
            },
        )?;

        Ok(Self {
            submission,
            status,
            submitted_at,
            version,
        })
    }

    /// Checks whether an idempotent retry carries the same normalized command.
    #[must_use]
    pub fn matches_submission(&self, candidate: &IssueSubmission) -> bool {
        self.submission.title == candidate.title
            && self.submission.category_id == candidate.category_id
            && self.submission.summary == candidate.summary
            && self.submission.problem_statement == candidate.problem_statement
            && self.submission.affected_community == candidate.affected_community
            && self.submission.desired_outcome == candidate.desired_outcome
            && self.submission.point == candidate.point
            && self.submission.geometry_source == candidate.geometry_source
            && self.submission.location_label == candidate.location_label
            && self.submission.public_attribution == candidate.public_attribution
            && self.submission.privacy_notice_version == candidate.privacy_notice_version
            && self.submission.submission_policy_version == candidate.submission_policy_version
    }

    #[must_use]
    pub const fn id(&self) -> IssueId {
        self.submission.id()
    }

    #[must_use]
    pub const fn reference(&self) -> IssueReference {
        self.submission.reference()
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.submission.title()
    }

    #[must_use]
    pub fn category_id(&self) -> &str {
        self.submission.category_id()
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        self.submission.summary()
    }

    #[must_use]
    pub fn problem_statement(&self) -> &str {
        self.submission.problem_statement()
    }

    #[must_use]
    pub fn affected_community(&self) -> &str {
        self.submission.affected_community()
    }

    #[must_use]
    pub fn desired_outcome(&self) -> &str {
        self.submission.desired_outcome()
    }

    #[must_use]
    pub const fn point(&self) -> GeoPoint {
        self.submission.point()
    }

    #[must_use]
    pub const fn geometry_source(&self) -> GeometrySource {
        self.submission.geometry_source()
    }

    #[must_use]
    pub fn location_label(&self) -> Option<&str> {
        self.submission.location_label()
    }

    #[must_use]
    pub const fn public_attribution(&self) -> bool {
        self.submission.public_attribution()
    }

    #[must_use]
    pub fn privacy_notice_version(&self) -> &str {
        self.submission.privacy_notice_version()
    }

    #[must_use]
    pub fn submission_policy_version(&self) -> &str {
        self.submission.submission_policy_version()
    }

    #[must_use]
    pub const fn status(&self) -> IssueStatus {
        self.status
    }

    #[must_use]
    pub const fn submitted_at(&self) -> DateTime<Utc> {
        self.submitted_at
    }

    #[must_use]
    pub const fn version(&self) -> u64 {
        self.version
    }
}

/// Safe field identifiers for validation errors. Rejected values are never
/// carried by the error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueField {
    Title,
    CategoryId,
    Summary,
    ProblemStatement,
    AffectedCommunity,
    DesiredOutcome,
    LocationLabel,
    PrivacyNoticeVersion,
    SubmissionPolicyVersion,
}

impl fmt::Display for IssueField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Title => "title",
            Self::CategoryId => "category_id",
            Self::Summary => "summary",
            Self::ProblemStatement => "problem_statement",
            Self::AffectedCommunity => "affected_community",
            Self::DesiredOutcome => "desired_outcome",
            Self::LocationLabel => "location_label",
            Self::PrivacyNoticeVersion => "privacy_notice_version",
            Self::SubmissionPolicyVersion => "submission_policy_version",
        })
    }
}

/// Domain validation failures without rejected citizen content or coordinates.
#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub enum IssueValidationError {
    #[error("{0} must not be blank")]
    BlankField(IssueField),
    #[error("{field} exceeds the {max} character limit")]
    FieldTooLong { field: IssueField, max: usize },
    #[error("category_id must use lowercase letters, digits, and single hyphens")]
    InvalidCategory,
    #[error("the confirmed point contains an invalid coordinate")]
    InvalidCoordinate,
    #[error("the problem location must be explicitly confirmed")]
    LocationNotConfirmed,
    #[error("the privacy notice must be accepted")]
    PrivacyNoticeNotAccepted,
    #[error("the persisted issue version is invalid")]
    InvalidVersion,
}

fn required_text(
    value: String,
    field: IssueField,
    max: usize,
) -> Result<String, IssueValidationError> {
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(IssueValidationError::BlankField(field));
    }
    if value.chars().count() > max {
        return Err(IssueValidationError::FieldTooLong { field, max });
    }
    Ok(value)
}

fn optional_text(
    value: Option<String>,
    field: IssueField,
    max: usize,
) -> Result<Option<String>, IssueValidationError> {
    value
        .map(|value| required_text(value, field, max))
        .transpose()
}

fn category_id(value: String) -> Result<String, IssueValidationError> {
    let value = required_text(value, IssueField::CategoryId, CATEGORY_MAX_CHARS)?;
    let valid = value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--");
    if !valid {
        return Err(IssueValidationError::InvalidCategory);
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        GeoPoint, GeometrySource, IssueField, IssueStatus, IssueSubmission, IssueSubmissionInput,
        IssueValidationError,
    };

    fn valid_input() -> IssueSubmissionInput {
        IssueSubmissionInput {
            title: "  Deep pothole  ".to_owned(),
            category_id: "road-surface".to_owned(),
            summary: "Buses avoid the damaged lane.".to_owned(),
            problem_statement: "The depression remains after repeated observations.".to_owned(),
            affected_community: "Passengers, cyclists, and drivers.".to_owned(),
            desired_outcome: "Restore a level and safe road surface.".to_owned(),
            point: match GeoPoint::new(-46.633_308, -23.550_52) {
                Ok(point) => point,
                Err(error) => panic!("valid fixture point failed: {error}"),
            },
            geometry_source: GeometrySource::MapSelection,
            location_confirmed: true,
            location_label: Some("  Eastbound bus lane  ".to_owned()),
            public_attribution: false,
            privacy_notice_version: "privacy-v1".to_owned(),
            privacy_notice_accepted: true,
        }
    }

    #[test]
    fn normalizes_valid_submission_without_exposing_internal_time_id() {
        let submission = IssueSubmission::new(valid_input());

        match submission {
            Ok(submission) => {
                assert_eq!(submission.title(), "Deep pothole");
                assert_eq!(submission.location_label(), Some("Eastbound bus lane"));
                assert_eq!(
                    submission.submission_policy_version(),
                    "issue-submission-v1"
                );
                assert_eq!(submission.reference().into_uuid().get_version_num(), 4);
                assert_eq!(submission.id().into_uuid().get_version_num(), 7);
            }
            Err(error) => panic!("valid submission failed: {error}"),
        }
    }

    #[test]
    fn rejects_unconfirmed_location() {
        let mut input = valid_input();
        input.location_confirmed = false;

        assert_eq!(
            IssueSubmission::new(input).err(),
            Some(IssueValidationError::LocationNotConfirmed)
        );
    }

    #[test]
    fn rejects_blank_text_without_retaining_the_value() {
        let mut input = valid_input();
        input.title = "   ".to_owned();

        assert_eq!(
            IssueSubmission::new(input).err(),
            Some(IssueValidationError::BlankField(IssueField::Title))
        );
    }

    #[test]
    fn rejects_invalid_coordinate() {
        assert_eq!(
            GeoPoint::new(181.0, 0.0).err(),
            Some(IssueValidationError::InvalidCoordinate)
        );
    }

    #[test]
    fn parses_all_initial_statuses() {
        for (value, expected) in [
            ("submitted", IssueStatus::Submitted),
            ("in_moderation", IssueStatus::InModeration),
            ("needs_clarification", IssueStatus::NeedsClarification),
            ("published", IssueStatus::Published),
            ("rejected", IssueStatus::Rejected),
        ] {
            assert_eq!(value.parse::<IssueStatus>(), Ok(expected));
        }
    }
}
