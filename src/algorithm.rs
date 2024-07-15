use digest::Digest;
use std::fmt::Display;

#[derive(Debug, PartialEq)]
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
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            inner: algorithm,
            marker: std::marker::PhantomData,
        }
    }

    pub fn name(&self) -> &str {
        self.inner.name()
    }

    pub fn algorithm(&self) -> &Algorithm {
        &self.inner
    }
}
