//! Framework-independent PowerTo domain types and rules.
//!
//! This crate must not depend on HTTP, persistence, telemetry, or cloud SDKs.

mod identity;
mod issues;

pub use identity::AccountId;
pub use issues::{
    CURRENT_SUBMISSION_POLICY, GeoPoint, GeometrySource, Issue, IssueField, IssueId,
    IssueReference, IssueStatus, IssueStatusParseError, IssueSubmission, IssueSubmissionInput,
    IssueValidationError,
};
