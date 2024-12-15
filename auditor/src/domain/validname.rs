// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::ValidationError;
use anyhow::Context;
use std::fmt;
use unicode_segmentation::UnicodeSegmentation;

// never turn this into `ValidName(pub String)`. By keeping the inner field private, it is not
// possible to create this type outside of this module, hence enforcing the use of `parse`. This
// ensures that every string stored in this type satisfies the validation criteria checked by
// `parse`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub struct ValidName(String);

impl ValidName {
    /// Returns `ValidName` only if input satisfies validation criteria, otherwise panics.
    pub fn parse(s: String) -> Result<ValidName, ValidationError> {
        // remove trailing whitespace and check if string is then empty
        let is_empty_or_whitespace = s.trim().is_empty();
        // count characters
        let is_too_long = s.graphemes(true).count() > 256;
        if is_empty_or_whitespace || is_too_long {
            Err(ValidationError(format!("Invalid Name: {s}")))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for ValidName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for ValidName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ValidName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf = String::deserialize(deserializer)?;
        // Aah I don't like this clone at all... If stuff is slow, figure this out.
        // I could remove the context, but it's nice to inform the user what's wrong. On the other
        // hand, if users use our clients, this parsing can't fail.
        ValidName::parse(buf.clone())
            .with_context(|| format!("Parsing '{buf}' failed"))
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ValidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::ValidName;
    use claim::{assert_err, assert_ok};
    use fake::{Fake, StringFaker};

    #[derive(Debug, Clone)]
    struct ValidNameString(pub String);

    impl quickcheck::Arbitrary for ValidNameString {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            let name = StringFaker::with(
                String::from(
                    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789*&^%$#@!~",
                )
                .into_bytes(),
                1..256,
            )
            .fake();
            Self(name)
        }
    }

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "ё".repeat(256);
        assert_ok!(ValidName::parse(name));
    }

    #[test]
    fn a_name_longer_than_256_graphemes_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(ValidName::parse(name));
    }

    #[test]
    fn whitespace_only_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(ValidName::parse(name));
    }

    #[test]
    fn empty_string_is_rejected() {
        let name = "".to_string();
        assert_err!(ValidName::parse(name));
    }

    #[quickcheck]
    fn a_valid_name_is_parsed_successfully(name: ValidNameString) {
        assert_ok!(ValidName::parse(name.0));
    }
}
