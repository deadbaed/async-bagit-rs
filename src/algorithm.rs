use digest::Digest;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// List of common hashing algorithms
///
/// Taken from <https://www.iana.org/assignments/named-information/named-information.xhtml>
pub enum Algorithm {
    Sha256,
    Sha512,
    Blake2b256,
    Blake2b512,
    Custom(&'static str),
}

impl Algorithm {
    /// Returns name of the algorithm, used in the filenames of the manifests files with checksums
    /// of said algorithm name
    pub fn name(&self) -> &str {
        match self {
            Algorithm::Sha256 => "sha256",
            Algorithm::Sha512 => "sha512",
            Algorithm::Blake2b256 => "blake2b256",
            Algorithm::Blake2b512 => "blake2b512",
            Algorithm::Custom(x) => x,
        }
    }
}

impl Display for Algorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, PartialEq)]
pub struct ChecksumAlgorithm<ChecksumAlgo: Digest> {
    inner: Algorithm,
    marker: std::marker::PhantomData<ChecksumAlgo>,
}

impl<ChecksumAlgo: Digest> ChecksumAlgorithm<ChecksumAlgo> {
    /// Link an algorithm enum variant with the type computing digests
    ///
    /// # Examples
    ///
    /// ```
    /// # use async_bagit::{Algorithm, ChecksumAlgorithm};
    /// let algorithm = ChecksumAlgorithm::<sha2::Sha256>::new(Algorithm::Sha256);
    /// ```
    ///
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            inner: algorithm,
            marker: std::marker::PhantomData,
        }
    }

    /// Shortcut to get name of the Algorithm. See [Algorithm::name()]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get a reference on the [Algorithm] enum.
    pub fn algorithm(&self) -> &Algorithm {
        &self.inner
    }
}
