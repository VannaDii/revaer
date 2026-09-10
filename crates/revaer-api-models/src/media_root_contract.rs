//! Input grammar for ADR 557's root-catalog and discovery HTTP contract.
//!
//! Validation preserves exact bytes: it never trims, normalizes, resolves, or
//! grants filesystem authority. A well-formed cursor still needs active-catalog
//! membership validation by the stored procedure. These helpers do not expose
//! routes or replace the legacy workflow before the coordinated init cutover.
//!
//! ```
//! use revaer_api_models::media_root_contract::{
//!     RootCatalogCursor, validate_root_association_prefix,
//!     validate_root_candidate_path, validate_root_catalog_limit,
//! };
//! use uuid::Uuid;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! validate_root_association_prefix("")?; // Explicitly select the whole root.
//! validate_root_candidate_path("season-1/episode.mkv")?;
//! assert!(validate_root_candidate_path("../episode.mkv").is_err());
//! let cursor = RootCatalogCursor::new("library", Uuid::from_u128(1))?;
//! assert_eq!(RootCatalogCursor::decode(&cursor.encode()?)?, cursor);
//! assert_eq!(validate_root_catalog_limit(None)?, 50);
//! # Ok(())
//! # }
//! ```

use std::{error::Error, fmt};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use uuid::Uuid;

const LOGICAL_KEY_MAX_BYTES: usize = 64;
const RELATIVE_PATH_MAX_BYTES: usize = 4_096;
const CURSOR_PREFIX_BYTES: usize = 2;
const CURSOR_UUID_BYTES: usize = 16;
const CURSOR_MAX_BYTES: usize = CURSOR_PREFIX_BYTES + LOGICAL_KEY_MAX_BYTES + CURSOR_UUID_BYTES;
const CURSOR_MAX_ENCODED_BYTES: usize = 110;

/// An invalid root input, without the rejected value or any filesystem identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootInputError;

impl fmt::Display for RootInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("media root input is invalid")
    }
}

impl Error for RootInputError {}

/// Validate one exact lowercase ASCII root or association key, without trimming.
///
/// # Errors
/// Rejects anything outside ADR 557's 1-64 byte alphanumeric/internal-hyphen
/// grammar. This is intentionally distinct from the legacy media-key grammar.
pub fn validate_root_logical_key(value: &str) -> Result<(), RootInputError> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > LOGICAL_KEY_MAX_BYTES
        || !bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes.last().is_some_and(u8::is_ascii_alphanumeric)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        return Err(RootInputError);
    }
    Ok(())
}

/// Validate a nonempty descriptor-relative candidate spelling, not its authority.
///
/// The caller must separately confine traversal to its attested descriptor.
/// Unicode and whitespace bytes are preserved; this is not a path normalizer.
///
/// # Errors
/// Rejects empty or over-4096-byte input, leading/trailing/repeated slash, NUL,
/// backslash, and any complete `.` or `..` component.
pub fn validate_root_candidate_path(value: &str) -> Result<(), RootInputError> {
    if value.is_empty()
        || value.len() > RELATIVE_PATH_MAX_BYTES
        || value.bytes().any(|byte| matches!(byte, 0 | b'\\'))
        || value
            .split('/')
            .any(|component| matches!(component, "" | "." | ".."))
    {
        return Err(RootInputError);
    }
    Ok(())
}

/// Validate an explicit association prefix; empty selects the whole attested root.
///
/// Omitted/null HTTP fields must be rejected by the request decoder, never
/// defaulted to empty. This helper accepts a present string only.
///
/// # Errors
/// A nonempty prefix must satisfy [`validate_root_candidate_path`].
pub fn validate_root_association_prefix(value: &str) -> Result<(), RootInputError> {
    if value.is_empty() {
        return Ok(());
    }
    validate_root_candidate_path(value)
}

/// Validate a parsed catalog page limit, defaulting an omitted limit to 50.
///
/// # Errors
/// Only 1 through 200 is accepted. Negative, noninteger, and out-of-`u16` HTTP
/// input must fail during query decoding rather than be coerced or defaulted.
pub const fn validate_root_catalog_limit(value: Option<u16>) -> Result<u16, RootInputError> {
    match value {
        None => Ok(50),
        Some(limit @ 1..=200) => Ok(limit),
        Some(_) => Err(RootInputError),
    }
}

/// A syntactically valid, resource-specific root-catalog continuation key.
///
/// This contains no attestation or generation authority. The database must
/// reject a key/UUID pair absent from the active catalog. Debug output omits both
/// values so diagnostics cannot accidentally expose catalog identifiers.
#[derive(Clone, PartialEq, Eq)]
pub struct RootCatalogCursor {
    logical_key: String,
    slot_public_id: Uuid,
}

impl RootCatalogCursor {
    /// Construct a cursor from a validated page's final key and public identity.
    ///
    /// # Errors
    /// Rejects a key outside [`validate_root_logical_key`]'s exact grammar.
    pub fn new(logical_key: &str, slot_public_id: Uuid) -> Result<Self, RootInputError> {
        validate_root_logical_key(logical_key)?;
        Ok(Self {
            logical_key: logical_key.to_owned(),
            slot_public_id,
        })
    }

    /// Decode exact, unpadded URL-safe Base64 of u16-BE key length, key, UUID.
    ///
    /// Decoding uses an 82-byte stack buffer and bounds the token before any
    /// decoding or owned-key allocation. No alternate or historical shape is
    /// accepted. UUID membership, including nil/unknown UUIDs, belongs to SQL.
    ///
    /// # Errors
    /// Rejects oversized, noncanonical, malformed, truncated, trailing, padded,
    /// or invalid-key encodings. The error never includes the token.
    pub fn decode(token: &str) -> Result<Self, RootInputError> {
        if token.len() > CURSOR_MAX_ENCODED_BYTES {
            return Err(RootInputError);
        }
        let mut buffer = [0_u8; CURSOR_MAX_BYTES];
        let length = URL_SAFE_NO_PAD
            .decode_slice(token, &mut buffer)
            .map_err(|_| RootInputError)?;
        let bytes = buffer.get(..length).ok_or(RootInputError)?;
        let prefix = bytes.get(..CURSOR_PREFIX_BYTES).ok_or(RootInputError)?;
        let key_length = usize::from(u16::from_be_bytes(
            prefix.try_into().map_err(|_| RootInputError)?,
        ));
        if !(1..=LOGICAL_KEY_MAX_BYTES).contains(&key_length)
            || length != CURSOR_PREFIX_BYTES + key_length + CURSOR_UUID_BYTES
        {
            return Err(RootInputError);
        }
        let key_end = CURSOR_PREFIX_BYTES + key_length;
        let key = bytes
            .get(CURSOR_PREFIX_BYTES..key_end)
            .ok_or(RootInputError)?;
        let identity = bytes.get(key_end..).ok_or(RootInputError)?;
        let key = std::str::from_utf8(key).map_err(|_| RootInputError)?;
        let identity = Uuid::from_slice(identity).map_err(|_| RootInputError)?;
        let cursor = Self::new(key, identity)?;
        if cursor.encode()? != token {
            return Err(RootInputError);
        }
        Ok(cursor)
    }

    /// Encode the single canonical representation, without padding or newlines.
    ///
    /// # Errors
    /// Returns an error if a private length invariant cannot be represented.
    pub fn encode(&self) -> Result<String, RootInputError> {
        let key = self.logical_key.as_bytes();
        let mut buffer = [0_u8; CURSOR_MAX_BYTES];
        let length = u16::try_from(key.len()).map_err(|_| RootInputError)?;
        buffer
            .get_mut(..CURSOR_PREFIX_BYTES)
            .ok_or(RootInputError)?
            .copy_from_slice(&length.to_be_bytes());
        let key_end = CURSOR_PREFIX_BYTES + key.len();
        buffer
            .get_mut(CURSOR_PREFIX_BYTES..key_end)
            .ok_or(RootInputError)?
            .copy_from_slice(key);
        let end = key_end + CURSOR_UUID_BYTES;
        buffer
            .get_mut(key_end..end)
            .ok_or(RootInputError)?
            .copy_from_slice(self.slot_public_id.as_bytes());
        Ok(URL_SAFE_NO_PAD.encode(buffer.get(..end).ok_or(RootInputError)?))
    }

    /// Return the exact key for the stored procedure's active-membership check.
    #[must_use]
    pub fn logical_key(&self) -> &str {
        &self.logical_key
    }

    /// Return the exact public slot identity for the same membership check.
    #[must_use]
    pub const fn slot_public_id(&self) -> Uuid {
        self.slot_public_id
    }
}

impl fmt::Debug for RootCatalogCursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootCatalogCursor")
    }
}

#[cfg(test)]
mod tests;
