#[cfg(feature = "date")]
use jiff::civil::Date;

use std::{borrow::Cow, fmt::Display, str::FromStr};

pub const KEY_VERSION: &str = "BagIt-Version";
pub const KEY_ENCODING: &str = "Tag-File-Character-Encoding";
pub const KEY_DATE: &str = "Bagging-Date";
pub const KEY_OXUM: &str = "Payload-Oxum";

#[derive(Debug, PartialEq)]
pub enum Metadata<'a> {
    Custom {
        key: Cow<'a, str>,
        value: Cow<'a, str>,
    },
    BagitVersion {
        major: u8,
        minor: u8,
    },
    Encoding,
    #[cfg(feature = "date")]
    BaggingDate(Date),
    PayloadOctetStreamSummary {
        /// Count of bytes in all streams
        octet_count: usize,
        /// Number of streams (aka files)
        stream_count: usize,
    },
}

impl Metadata<'_> {
    pub fn key(&self) -> &str {
        match self {
            Metadata::Custom { key, .. } => key,
            Metadata::BagitVersion { .. } => KEY_VERSION,
            Metadata::Encoding => KEY_ENCODING,
            #[cfg(feature = "date")]
            Metadata::BaggingDate(_) => KEY_DATE,
            Metadata::PayloadOctetStreamSummary { .. } => KEY_OXUM,
        }
    }

    pub fn value(&self) -> String {
        match self {
            Metadata::Custom { value, .. } => value.to_string(),
            Metadata::BagitVersion { major, minor } => format!("{major}.{minor}"),
            Metadata::Encoding => "UTF-8".to_string(),
            #[cfg(feature = "date")]
            Metadata::BaggingDate(date) => date.to_string(),
            Metadata::PayloadOctetStreamSummary {
                octet_count,
                stream_count,
            } => format!("{octet_count}.{stream_count}"),
        }
    }
}

impl Display for Metadata<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.key(), self.value())
    }
}

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum MetadataError {
    /// Metadata format must be: "<key>: <value>"
    #[error("Invalid format")]
    Format,
    /// Some characters are forbidden for labels
    #[error("Metadata key contains forbidden character `:`")]
    KeyForbiddenCharacter,
    /// Some characters are forbidden for values
    #[error("Metadata value contains forbidden character `<whitespace>`")]
    ValueForbiddenCharacter,
    /// Some characters are forbidden for values
    #[error("Failed to parse metadata value for key `{0}`")]
    ValueParsing(&'static str),
    /// Got other encoding value, accepting only utf-8
    #[error("Only UTF-8 is supported")]
    Encoding,
}

impl FromStr for Metadata<'_> {
    type Err = MetadataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, value) = s.split_once(": ").ok_or(MetadataError::Format)?;

        Self::validate_format(key, value)?;

        Ok(match (key, value) {
            (KEY_VERSION, version) => {
                let (major, minor) = version
                    .split_once(".")
                    .ok_or(MetadataError::ValueParsing(KEY_VERSION))?;

                let major = major
                    .parse()
                    .map_err(|_| MetadataError::ValueParsing(KEY_VERSION))?;
                let minor = minor
                    .parse()
                    .map_err(|_| MetadataError::ValueParsing(KEY_VERSION))?;

                Metadata::BagitVersion { major, minor }
            }
            (KEY_ENCODING, encoding) => {
                if encoding != "UTF-8" {
                    return Err(MetadataError::Encoding);
                }

                Metadata::Encoding
            }
            #[cfg(feature = "date")]
            (KEY_DATE, date) => {
                let date =
                    Date::from_str(date).map_err(|_| MetadataError::ValueParsing(KEY_DATE))?;

                Metadata::BaggingDate(date)
            }
            (KEY_OXUM, oxum) => {
                let (octet_count, stream_count) = oxum
                    .split_once(".")
                    .ok_or(MetadataError::ValueParsing(KEY_OXUM))?;

                let octet_count = octet_count
                    .parse()
                    .map_err(|_| MetadataError::ValueParsing(KEY_OXUM))?;
                let stream_count = stream_count
                    .parse()
                    .map_err(|_| MetadataError::ValueParsing(KEY_OXUM))?;

                Metadata::PayloadOctetStreamSummary {
                    octet_count,
                    stream_count,
                }
            }
            (_, _) => Metadata::Custom {
                key: Cow::Owned(key.to_string()),
                value: Cow::Owned(value.to_string()),
            },
        })
    }
}

impl Metadata<'_> {
    fn validate_format(key: &str, value: &str) -> Result<(), MetadataError> {
        if key.is_empty() || value.is_empty() {
            return Err(MetadataError::Format);
        }

        if key.contains(':') {
            return Err(MetadataError::KeyForbiddenCharacter);
        }

        if value.starts_with(char::is_whitespace) || value.ends_with(char::is_whitespace) {
            return Err(MetadataError::ValueForbiddenCharacter);
        }

        Ok(())
    }
}

impl<'a> Metadata<'a> {
    pub fn custom(key: &'a str, value: &'a str) -> Result<Self, MetadataError> {
        Self::validate_format(key, value)?;

        Ok(Self::Custom {
            key: Cow::Borrowed(key),
            value: Cow::Borrowed(value),
        })
    }
}

#[cfg(test)]
mod test {
    use super::{Metadata, MetadataError};
    use jiff::civil::Date;
    use std::str::FromStr;

    #[test]
    fn detect_key() {
        for (input, output) in [
            (
                "Custom-Tag: Custom value",
                Ok(Metadata::Custom {
                    key: "Custom-Tag".into(),
                    value: "Custom value".into(),
                }),
            ),
            (
                "BagIt-Version: 43.69",
                Ok(Metadata::BagitVersion {
                    major: 43,
                    minor: 69,
                }),
            ),
            ("Tag-File-Character-Encoding: UTF-8", Ok(Metadata::Encoding)),
            #[cfg(feature = "date")]
            (
                "Bagging-Date: 2024-07-28 17:48",
                Ok(Metadata::BaggingDate(Date::new(2024, 7, 28).unwrap())),
            ),
            (
                "Payload-Oxum: 420.69",
                Ok(Metadata::PayloadOctetStreamSummary {
                    octet_count: 420,
                    stream_count: 69,
                }),
            ),
        ] {
            assert_eq!(
                Metadata::from_str(input),
                output,
                "failing on input value `{input}`"
            );
        }
    }

    #[cfg(feature = "date")]
    #[test]
    fn bagging_date() {
        let date = Date::new(2024, 7, 28).unwrap();
        let bagging_date = Metadata::BaggingDate(date);

        assert_eq!(bagging_date.key(), "Bagging-Date");
        assert_eq!(bagging_date.value(), "2024-07-28");
        assert_eq!(bagging_date.to_string(), "Bagging-Date: 2024-07-28");
    }

    #[test]
    fn custom_from_str() {
        for (input, output) in [
            ("lolwrongformat", Err(MetadataError::Format)),
            ("still wrong format", Err(MetadataError::Format)),
            ("almost there", Err(MetadataError::Format)),
            (
                "Bad:Tag:      Bad Value    ",
                Err(MetadataError::KeyForbiddenCharacter),
            ),
            (
                "Bad:Tag: GoodValue",
                Err(MetadataError::KeyForbiddenCharacter),
            ),
            (
                "Good-Tag: \tBad   Value\n \t",
                Err(MetadataError::ValueForbiddenCharacter),
            ),
            (
                "Good-Tag: Good Value",
                Ok(Metadata::Custom {
                    key: "Good-Tag".into(),
                    value: "Good Value".into(),
                }),
            ),
        ] {
            assert_eq!(
                Metadata::from_str(input),
                output,
                "failing on input value `{input}`"
            );
        }
    }

    #[test]
    fn new_custom() {
        for (key, value, output) in [
            ("tag", "", Err(MetadataError::Format)),
            ("", "value", Err(MetadataError::Format)),
            (
                "Bad:Tag",
                "    Bad Value    ",
                Err(MetadataError::KeyForbiddenCharacter),
            ),
            (
                "Bad:Tag",
                "GoodValue",
                Err(MetadataError::KeyForbiddenCharacter),
            ),
            (
                "Good-Tag",
                "\tBad   Value\n \t",
                Err(MetadataError::ValueForbiddenCharacter),
            ),
            (
                "Good-Tag",
                "Good Value",
                Ok(Metadata::Custom {
                    key: "Good-Tag".into(),
                    value: "Good Value".into(),
                }),
            ),
        ] {
            assert_eq!(
                Metadata::custom(key, value),
                output,
                "failing with key `{key}` and value `{value}`"
            );
        }
    }

    #[test]
    fn custom() {
        let custom = Metadata::Custom {
            key: "Unusual-But-Correct-Tag".into(),
            value: "Unexpected but good value".into(),
        };

        assert_eq!(custom.key(), "Unusual-But-Correct-Tag");
        assert_eq!(custom.value(), "Unexpected but good value");
        assert_eq!(
            custom.to_string(),
            "Unusual-But-Correct-Tag: Unexpected but good value"
        );
    }
}
