//! Linux mountinfo observations, never deployment or durability authority.

use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Path-free failure when mount observations cannot support a root identity.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub enum RootMountError {
    /// Missing fields, invalid identifiers, paths or escaping.
    #[error("root mount observation is invalid")]
    Invalid,
    /// A mount ID occurs more than once in the supplied snapshot.
    #[error("root mount observation contains duplicate identities")]
    Duplicate,
    /// The descriptor mount/device/path does not match the snapshot.
    #[error("root mount observation differs from the retained descriptor")]
    IdentityMismatch,
    /// Distinct slots resolve to overlapping regions of one filesystem.
    #[error("root mount aliases overlap")]
    Overlap,
}

struct Mount {
    device: (u32, u32),
    root: PathBuf,
    point: PathBuf,
    filesystem: String,
}

/// Parsed Linux mountinfo snapshot supplied by the bootstrap observer.
///
/// Parsing alone proves neither provenance nor freshness. Callers must supply
/// the service's own mount namespace snapshot and repeat observation around
/// attestation/admission. No supported filesystem or durability is inferred.
pub struct RootMountTopology {
    mounts: BTreeMap<u64, Mount>,
}

impl fmt::Debug for RootMountTopology {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RootMountTopology")
    }
}

impl RootMountTopology {
    /// Whether a descriptor's observed mount is separate from the image/root mount.
    /// This topology fact alone does not establish persistence or writer ownership.
    ///
    /// # Errors
    /// Rejects an identity absent from the supplied current namespace snapshot.
    pub fn is_external_mount(&self, id: u64) -> Result<bool, RootMountError> {
        let mount = self
            .mounts
            .get(&id)
            .ok_or(RootMountError::IdentityMismatch)?;
        let root = self
            .mounts
            .values()
            .find(|entry| entry.point == Path::new("/"))
            .ok_or(RootMountError::IdentityMismatch)?;
        Ok(mount.point != Path::new("/") && mount.device != root.device)
    }

    /// Parse the kernel mountinfo field layout, including optional-field separator.
    /// Unknown optional fields are ignored as required by the Linux contract.
    ///
    /// # Errors
    /// Rejects malformed/duplicate records and unsupported path encodings.
    pub fn parse(input: &str) -> Result<Self, RootMountError> {
        let mut mounts = BTreeMap::new();
        for line in input.lines() {
            let (id, mount) = parse_line(line)?;
            if mounts.insert(id, mount).is_some() {
                return Err(RootMountError::Duplicate);
            }
        }
        if mounts.is_empty() {
            return Err(RootMountError::Invalid);
        }
        Ok(Self { mounts })
    }

    pub(super) fn resolve(
        &self,
        id: u64,
        device: (u32, u32),
        path: &Path,
    ) -> Result<MountLocation, RootMountError> {
        let mount = self
            .mounts
            .get(&id)
            .ok_or(RootMountError::IdentityMismatch)?;
        if mount.device != device {
            return Err(RootMountError::IdentityMismatch);
        }
        let relative = path
            .strip_prefix(&mount.point)
            .map_err(|_| RootMountError::IdentityMismatch)?;
        Ok(MountLocation {
            device,
            path: mount.root.join(relative),
            filesystem: mount.filesystem.clone(),
        })
    }

    pub(super) fn regions(
        &self,
        id: u64,
        device: (u32, u32),
        path: &Path,
    ) -> Result<Vec<MountLocation>, RootMountError> {
        let mut regions = vec![self.resolve(id, device, path)?];
        // Include every descendant record, even a covered mount: an ambiguous
        // namespace must not make physical overlap disappear from attestation.
        for mount in self.mounts.values() {
            if mount.point != path && mount.point.starts_with(path) {
                regions.push(MountLocation {
                    device: mount.device,
                    path: mount.root.clone(),
                    filesystem: mount.filesystem.clone(),
                });
            }
        }
        Ok(regions)
    }
}

pub(super) struct MountLocation {
    device: (u32, u32),
    path: PathBuf,
    filesystem: String,
}

impl MountLocation {
    pub(super) fn filesystem_type(&self) -> &str {
        &self.filesystem
    }

    pub(super) fn reject_overlap(&self, other: &Self) -> Result<(), RootMountError> {
        if self.device == other.device {
            if self.filesystem != other.filesystem {
                return Err(RootMountError::IdentityMismatch);
            }
            if self.path.starts_with(&other.path) || other.path.starts_with(&self.path) {
                return Err(RootMountError::Overlap);
            }
        }
        Ok(())
    }
}

fn parse_line(line: &str) -> Result<(u64, Mount), RootMountError> {
    let fields: Vec<_> = line.split(' ').collect();
    let separator = fields
        .iter()
        .position(|field| *field == "-")
        .ok_or(RootMountError::Invalid)?;
    if separator < 6 || fields.len() != separator + 4 || fields.contains(&"") {
        return Err(RootMountError::Invalid);
    }
    let id = identifier(fields[0])?;
    identifier(fields[1])?;
    let (major, minor) = fields[2].split_once(':').ok_or(RootMountError::Invalid)?;
    let device = (device_part(major)?, device_part(minor)?);
    let filesystem = fields[separator + 1];
    if filesystem.len() > 64 || filesystem.contains('\0') {
        return Err(RootMountError::Invalid);
    }
    Ok((
        id,
        Mount {
            device,
            root: decode_path(fields[3])?,
            point: decode_path(fields[4])?,
            filesystem: filesystem.into(),
        },
    ))
}

fn identifier(value: &str) -> Result<u64, RootMountError> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RootMountError::Invalid);
    }
    let parsed = value.parse::<u64>().map_err(|_| RootMountError::Invalid)?;
    i64::try_from(parsed).map_err(|_| RootMountError::Invalid)?;
    Ok(parsed)
}

fn device_part(value: &str) -> Result<u32, RootMountError> {
    u32::try_from(identifier(value)?).map_err(|_| RootMountError::Invalid)
}

fn decode_path(value: &str) -> Result<PathBuf, RootMountError> {
    let mut decoded = Vec::with_capacity(value.len());
    let mut bytes = value.bytes();
    while let Some(byte) = bytes.next() {
        if byte == b'\\' {
            let escape = [bytes.next(), bytes.next(), bytes.next()];
            decoded.push(match escape {
                [Some(b'0'), Some(b'4'), Some(b'0')] => b' ',
                [Some(b'0'), Some(b'1'), Some(b'1')] => b'\t',
                [Some(b'0'), Some(b'1'), Some(b'2')] => b'\n',
                [Some(b'1'), Some(b'3'), Some(b'4')] => b'\\',
                _ => return Err(RootMountError::Invalid),
            });
        } else {
            decoded.push(byte);
        }
    }
    let decoded = String::from_utf8(decoded).map_err(|_| RootMountError::Invalid)?;
    if !decoded.starts_with('/')
        || decoded.contains('\0')
        || (decoded != "/"
            && decoded[1..]
                .split('/')
                .any(|part| matches!(part, "" | "." | "..")))
    {
        return Err(RootMountError::Invalid);
    }
    Ok(decoded.into())
}

#[cfg(test)]
mod tests;
