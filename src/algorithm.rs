use digest::Digest;
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// List of common hashing algorithms
///
/// The string representation of the algorithm is used in the filename of manifest files.

/// This list was taken from <https://www.iana.org/assignments/named-information/named-information.xhtml>, but it is not exhaustive, as new secure algorithms come, and old ones get broken.
pub enum Algorithm {
    /// Secure Hash Algorithm 2 hash function with 32-bit words
    Sha256,
    /// Secure Hash Algorithm 2 hash function with 64-bit words
    Sha512,
    /// BLAKE2 hash function with 32-bit words
    Blake2b256,
    /// BLAKE2 hash function with 64-bit words
    Blake2b512,
    /// Custom hash function
    Custom(&'static str),
}

impl Algorithm {
    /// Returns name of the algorithm, used in the filenames of the manifests files with checksums
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
/// Wrapper around the [`Algorithm`] enum that associates a specific hashing algorithm with a concrete type computing digests.
///
/// This struct is generic over a concrete type that implements [`Digest`] trait.
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
    /// // Sha256, commonly used algorithm
    /// let algorithm = ChecksumAlgorithm::<sha2::Sha256>::new(Algorithm::Sha256);
    ///
    /// // BLAKE3, a bit less known algorithm
    /// let algorithm = ChecksumAlgorithm::<blake3::Hasher>::new(Algorithm::Custom("blake3"));
    /// ```
    ///
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            inner: algorithm,
            marker: std::marker::PhantomData,
        }
    }

    /// Shortcut to get name of the Algorithm. See [`Algorithm::name()`]
    pub fn name(&self) -> &str {
        self.inner.name()
    }

    /// Get a reference on the [`Algorithm`] enum.
    pub fn algorithm(&self) -> &Algorithm {
        &self.inner
    }
}
