//! Extensions for [`cap_std::fs::Dir`].
//!
//! [`cap_std::fs::Dir`]: https://docs.rs/cap-std/latest/cap_std/fs/struct.Dir.html

use crate::tempfile::LinkableTempfile;
use cap_std::fs::{Dir, File, Metadata};
use std::io::Result;
use std::path::Path;

/// Extension trait for [`cap_std::fs::Dir`]
pub trait CapStdExtDirExt {
    /// Open a file read-only, but return `Ok(None)` if it does not exist.
    fn open_optional(&self, path: impl AsRef<Path>) -> Result<Option<File>>;

    /// Open a directory, but return `Ok(None)` if it does not exist.
    fn open_dir_optional(&self, path: impl AsRef<Path>) -> Result<Option<Dir>>;

    /// Gather metadata, but return `Ok(None)` if it does not exist.
    fn metadata_optional(&self, path: impl AsRef<Path>) -> Result<Option<Metadata>>;

    /// Remove (delete) a file, but return `Ok(false)` if the file does not exist.
    fn remove_file_optional(&self, path: impl AsRef<Path>) -> Result<bool>;

    /// Create a new anonymous file that can be given a persistent name.
    /// On Linux, this uses `O_TMPFILE` if possible, otherwise a randomly named
    /// temporary file is used.  
    ///
    /// The file can later be linked into place once it has been completely written.
    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn new_linkable_file<'p, 'd>(&'d self, path: &'p Path) -> Result<LinkableTempfile<'p, 'd>>;

    /// Atomically write a file, replacing an existing one (if present).
    ///
    /// This wraps [`Self::new_linkable_file`] and [`crate::tempfile::LinkableTempfile::replace`].
    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn replace_file_with<F, T, E>(
        &self,
        destname: impl AsRef<Path>,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut std::io::BufWriter<LinkableTempfile>) -> std::result::Result<T, E>,
        E: From<std::io::Error>;

    /// Atomically write a file using specified permissions, replacing an existing one (if present).
    ///
    /// This wraps [`Self::new_linkable_file`] and [`crate::tempfile::LinkableTempfile::replace_with_perms`].
    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn replace_file_with_perms<F, T, E>(
        &self,
        destname: impl AsRef<Path>,
        perms: cap_std::fs::Permissions,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut std::io::BufWriter<LinkableTempfile>) -> std::result::Result<T, E>,
        E: From<std::io::Error>;

    /// Atomically write a file contents using specified permissions, replacing an existing one (if present).
    ///
    /// This wraps [`Self::new_linkable_file`] and [`crate::tempfile::LinkableTempfile::replace_with_perms`].
    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn replace_contents_with_perms(
        &self,
        destname: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
        perms: cap_std::fs::Permissions,
    ) -> Result<()>;
}

fn map_optional<R>(r: Result<R>) -> Result<Option<R>> {
    match r {
        Ok(v) => Ok(Some(v)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

impl CapStdExtDirExt for Dir {
    fn open_optional(&self, path: impl AsRef<Path>) -> Result<Option<File>> {
        map_optional(self.open(path.as_ref()))
    }

    fn open_dir_optional(&self, path: impl AsRef<Path>) -> Result<Option<Dir>> {
        map_optional(self.open_dir(path.as_ref()))
    }

    fn metadata_optional(&self, path: impl AsRef<Path>) -> Result<Option<Metadata>> {
        map_optional(self.metadata(path.as_ref()))
    }

    fn remove_file_optional(&self, path: impl AsRef<Path>) -> Result<bool> {
        match self.remove_file(path.as_ref()) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn new_linkable_file<'p, 'd>(
        &'d self,
        target: &'p Path,
    ) -> Result<crate::tempfile::LinkableTempfile<'p, 'd>> {
        crate::tempfile::LinkableTempfile::new_in(self, target)
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn replace_file_with<F, T, E>(
        &self,
        destname: impl AsRef<Path>,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut std::io::BufWriter<LinkableTempfile>) -> std::result::Result<T, E>,
        E: From<std::io::Error>,
    {
        let t = self.new_linkable_file(destname.as_ref())?;
        let mut bufw = std::io::BufWriter::new(t);
        match f(&mut bufw) {
            Ok(r) => bufw
                .into_inner()
                .map_err(From::from)
                .and_then(|t| t.replace())
                .map(|()| r)
                .map_err(From::from),
            Err(e) => Err(e.into()),
        }
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    fn replace_file_with_perms<F, T, E>(
        &self,
        destname: impl AsRef<Path>,
        perms: cap_std::fs::Permissions,
        f: F,
    ) -> std::result::Result<T, E>
    where
        F: FnOnce(&mut std::io::BufWriter<LinkableTempfile>) -> std::result::Result<T, E>,
        E: From<std::io::Error>,
    {
        let t = self.new_linkable_file(destname.as_ref())?;
        let mut bufw = std::io::BufWriter::new(t);
        match f(&mut bufw) {
            Ok(r) => bufw
                .into_inner()
                .map_err(From::from)
                .and_then(|t| t.replace_with_perms(perms))
                .map(|()| r)
                .map_err(From::from),
            Err(e) => Err(e.into()),
        }
    }

    fn replace_contents_with_perms(
        &self,
        destname: impl AsRef<Path>,
        contents: impl AsRef<[u8]>,
        perms: cap_std::fs::Permissions,
    ) -> Result<()> {
        let t = self.new_linkable_file(destname.as_ref())?;
        t.replace_contents_using_perms(contents, perms)
    }
}
