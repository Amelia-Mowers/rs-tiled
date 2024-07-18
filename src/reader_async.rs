use std::{future::Future, path::Path};

use tokio::{
    fs::File,
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

/// An [`AsyncResourceReader`] that reads from Tokio [`File`] handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsyncFilesystemResourceReader;

impl AsyncFilesystemResourceReader {
    /// Creates a new [`AsyncFilesystemResourceReader`].
    pub fn new() -> Self {
        Self
    }
}

impl AsyncResourceReader for AsyncFilesystemResourceReader {
    type Resource = BufReader<File>;
    type Error = std::io::Error;

    async fn read_from(&mut self, path: &Path) -> std::result::Result<Self::Resource, Self::Error> {
        let file = File::open(path).await?;
        Ok(BufReader::new(file))
    }
}

impl<T, F, R, E> AsyncResourceReader for T
where
    T: for<'a> Fn(&'a Path) -> F,
    F: Future<Output = Result<R, E>>,
    R: AsyncBufRead + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    type Resource = R;

    type Error = E;

    // FIXME: remove `allow(refining_impl_trait)` or this comment or add #[refine]
    // this lint is under discussion at https://github.com/rust-lang/rust/issues/121718
    // eventually, we'd want to #[refine] this function if this attribute becomes available
    #[allow(refining_impl_trait)]
    fn read_from(&mut self, path: &Path) -> F {
        self(path)
    }
}
