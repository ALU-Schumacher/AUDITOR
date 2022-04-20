use sqlx::{postgres::PgTypeInfo, Postgres, Type};
use std::fmt;
use unicode_segmentation::UnicodeSegmentation;

// never turn this into `ValidName(pub String)`. By keeping the inner field private, it is not
// possible to create this type outside of this module, hence enforcing the use of `parse`. This
// ensures that every string stored in this type satisfies the validation criteria checked by
// `parse`.
#[derive(Debug, Clone, PartialEq, sqlx::Decode, sqlx::Encode)]
pub struct ValidName(String);

impl ValidName {
    /// Returns `ValidName` only if input satisfies validation criteria, otherwise panics.
    pub fn parse(s: String) -> Result<ValidName, String> {
        // remove trailing whitespace and check if string is then empty
        let is_empty_or_whitespace = s.trim().is_empty();
        // count characters
        let is_too_long = s.graphemes(true).count() > 256;
        // check for forbidden characters
        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));
        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("Invalid record ID: {}", s))
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

impl Type<Postgres> for ValidName {
    fn type_info() -> PgTypeInfo {
        <&str as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&str as Type<Postgres>>::compatible(ty)
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
        ValidName::parse(buf).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ValidName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
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
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let name = StringFaker::with(
                String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789*&^%$#@!~").into_bytes(),
                1..256,
            )
            .fake_with_rng::<String, G>(g);
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

    #[test]
    fn names_containing_an_invalid_character_are_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            let name = name.to_string();
            assert_err!(ValidName::parse(name));
        }
    }

    // #[test]
    #[quickcheck_macros::quickcheck]
    fn a_valid_name_is_parsed_successfully(name: ValidNameString) {
        // dbg!(&name.0);
        assert_ok!(ValidName::parse(name.0));
    }
}
