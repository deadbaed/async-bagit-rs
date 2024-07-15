pub(crate) use compute::compute_checksum_file;
pub use compute::ChecksumComputeError;
use digest::Digest;
use std::{borrow::Cow, fmt::Display};

mod compute {
    use super::Checksum;
    use digest::Digest;
    use std::path::Path;
    use tokio::{
        fs::File,
        io::{AsyncReadExt, BufReader},
        task::spawn_blocking,
    };

    #[derive(thiserror::Error, Debug, PartialEq)]
    pub enum ChecksumComputeError {
        #[error("File not found on disk")]
        FileNotFound,
        #[error("Failed to open file")]
        OpenFile(std::io::ErrorKind),
        #[error("Failed to read file")]
        ReadFile(std::io::ErrorKind),
        #[error("Failed to compute checksum of file")]
        ComputeChecksum,
    }

    pub(crate) async fn compute_checksum_file<ChecksumAlgo: Digest>(
        path: impl AsRef<Path>,
    ) -> Result<Checksum<'static>, ChecksumComputeError> {
        if !path.as_ref().is_file() {
            return Err(ChecksumComputeError::FileNotFound);
        }

        // Read file and verify checksum
        let file = File::open(&path)
            .await
            .map_err(|e| ChecksumComputeError::OpenFile(e.kind()))?;
        let mut buffer_reader = BufReader::new(file);

        // TODO: read file chunks by chunks?
        let mut buffer = Vec::new();
        buffer_reader
            .read_to_end(&mut buffer)
            .await
            .map_err(|e| ChecksumComputeError::ReadFile(e.kind()))?;

        let checksum = spawn_blocking(move || Checksum::digest::<ChecksumAlgo>(buffer))
            .await
            .map_err(|_| ChecksumComputeError::ComputeChecksum)?;

        Ok(checksum)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Checksum<'a>(Cow<'a, str>);

impl Checksum<'_> {
    /// Compute checksum for vector of bytes
    pub fn digest<Algorithm: Digest>(bytes: Vec<u8>) -> Self {
        Algorithm::digest(bytes).to_vec().into()
    }
}

impl From<&[u8]> for Checksum<'_> {
    fn from(value: &[u8]) -> Self {
        Self(Cow::Owned(hex::encode(value)))
    }
}

impl From<Vec<u8>> for Checksum<'_> {
    fn from(value: Vec<u8>) -> Self {
        Self(Cow::Owned(hex::encode(value)))
    }
}

impl<'a> From<&'a str> for Checksum<'a> {
    fn from(value: &'a str) -> Checksum<'a> {
        Self(Cow::Borrowed(value))
    }
}

impl From<String> for Checksum<'_> {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}

impl Display for Checksum<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Checksum<'_> {
    fn as_ref(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(borrowed) => borrowed,
            Cow::Owned(owned) => owned.as_ref(),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn compare() {
        let bytes: &[u8; 32] = &[
            214, 211, 134, 26, 157, 177, 72, 1, 68, 222, 226, 175, 114, 10, 93, 79, 34, 48, 98, 18,
            108, 223, 93, 138, 125, 83, 191, 237, 98, 51, 186, 189,
        ];

        let left =
            Checksum::from("d6d3861a9db1480144dee2af720a5d4f223062126cdf5d8a7d53bfed6233babd");
        let right = Checksum::from(bytes.as_ref());
        assert_eq!(left, right);
    }

    #[test]
    fn sha256() {
        assert_eq!(
            Checksum::digest::<sha2::Sha256>("i love my bag, it is awesome".into()),
            Checksum::from("9d5e40310ff9851f519fe3f84770e7c4ef9d840d26d040804db4a1fd0a9d4038")
        );
    }
}
