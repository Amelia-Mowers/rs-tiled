use std::{future::Future, path::Path};

use tokio::{
    io::{AsyncBufRead, BufReader},
};

/// A trait defining types that can asynchronously load data from a
/// [`ResourcePath`](crate::ResourcePath).
///
/// This is the async version of [`ResourceReader`](crate::reader::ResourceReader). It should be
/// implemented if you wish to asynchronously load data from a virtual filesystem.
pub trait AsyncResourceReader {
    /// The type of the resource that the reader provides. For example, for
    /// [`FilesystemResourceReader`], this is defined as [`File`].
    type Resource: AsyncBufRead + Unpin;

    /// The type that is returned if [`read_from()`](Self::read_from()) fails. For example, for
    /// [`FilesystemResourceReader`], this is defined as [`std::io::Error`].
    type Error: std::error::Error + Send + Sync + 'static;

    /// Try to return a reader object from a path into the resources filesystem.
    fn read_from(
        &mut self,
        path: &Path,
    ) -> impl Future<Output = Result<Self::Resource, Self::Error>>;
}
