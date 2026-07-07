//! Step-up reverification: prompting a signed-in user to re-assert a fresh
//! authentication factor before a sensitive action proceeds.
//!
//! This is distinct from token [`verification`](super::verification), which
//! checks a request's existing credentials. Reverification asks the user to
//! authenticate *again*, at a required factor [`ReverificationLevel`], before a
//! gated action runs.

use serde::{Deserialize, Serialize};

/// The authentication-factor level a step-up reverification requires, mirroring
/// clerk-js's `SessionVerificationLevel`.
///
/// Serializes as the raw clerk-js level string. The enum is
/// `#[non_exhaustive]`; levels this crate has not named yet round-trip through
/// [`ReverificationLevel::Other`], mirroring
/// [`SessionTaskKey`](super::SessionTaskKey).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
#[non_exhaustive]
pub enum ReverificationLevel {
    /// A single first-factor credential (e.g. password) must be re-verified.
    FirstFactor,
    /// A single second-factor credential (e.g. TOTP) must be re-verified.
    SecondFactor,
    /// Both a first and a second factor must be re-verified.
    MultiFactor,
    /// A level string this crate has not named yet.
    ///
    /// Only produced by the `From<&str>`/`From<String>`/`FromStr` conversions, which
    /// canonicalize known levels to their named variants first. The payload is
    /// an [`OtherReverificationLevel`] with no public constructor, so an `Other`
    /// can never alias a named variant.
    Other(OtherReverificationLevel),
}

/// A clerk-js reverification-level string with no named [`ReverificationLevel`]
/// variant.
///
/// Obtained by matching on [`ReverificationLevel::Other`]; read the raw string
/// with [`OtherReverificationLevel::as_str`]. It has no public constructor, so
/// it never holds a value a named variant would represent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OtherReverificationLevel(String);

impl OtherReverificationLevel {
    /// The raw clerk-js level string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for OtherReverificationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl ReverificationLevel {
    /// The raw clerk-js level string.
    pub fn as_str(&self) -> &str {
        match self {
            Self::FirstFactor => "first_factor",
            Self::SecondFactor => "second_factor",
            Self::MultiFactor => "multi_factor",
            Self::Other(level) => level.as_str(),
        }
    }

    fn from_known(level: &str) -> Option<Self> {
        Some(match level {
            "first_factor" => Self::FirstFactor,
            "second_factor" => Self::SecondFactor,
            "multi_factor" => Self::MultiFactor,
            _ => return None,
        })
    }
}

impl From<&str> for ReverificationLevel {
    fn from(level: &str) -> Self {
        Self::from_known(level)
            .unwrap_or_else(|| Self::Other(OtherReverificationLevel(level.to_owned())))
    }
}

impl From<String> for ReverificationLevel {
    fn from(level: String) -> Self {
        // Move the owned buffer into `Other` instead of re-allocating it.
        Self::from_known(&level).unwrap_or(Self::Other(OtherReverificationLevel(level)))
    }
}

impl From<ReverificationLevel> for String {
    fn from(level: ReverificationLevel) -> Self {
        level.as_str().to_owned()
    }
}

impl std::str::FromStr for ReverificationLevel {
    type Err = std::convert::Infallible;

    fn from_str(level: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(level))
    }
}

impl std::fmt::Display for ReverificationLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
