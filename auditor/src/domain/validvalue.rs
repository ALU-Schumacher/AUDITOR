// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::ValidationError;
use anyhow::Context;
use sqlx::{Postgres, Type, postgres::PgTypeInfo};
use std::fmt;

// never turn this into `ValidValue(pub f64)`. By keeping the inner field private, it is not
// possible to create this type outside of this module, hence enforcing the use of `parse`. This
// ensures that every string stored in this type satisfies the validation criteria checked by
// `parse`.
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Decode, sqlx::Encode)]
pub struct ValidValue(f64);

impl ValidValue {
    /// Returns `ValidValue` only if input satisfies validation criteria, otherwise panics.
    pub fn parse(s: f64) -> Result<ValidValue, ValidationError> {
        if s < 0.0 {
            Err(ValidationError(format!("Invalid value: {s}")))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<f64> for ValidValue {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl Type<Postgres> for ValidValue {
    fn type_info() -> PgTypeInfo {
        <&f64 as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&f64 as Type<Postgres>>::compatible(ty)
    }
}

impl serde::Serialize for ValidValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ValidValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf = f64::deserialize(deserializer)?;
        ValidValue::parse(buf)
            .with_context(|| format!("Parsing '{buf}' failed."))
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ValidValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::ValidValue;
    use claim::{assert_err, assert_ok};
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidValueF64(pub f64);

    impl quickcheck::Arbitrary for ValidValueF64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((0.0..f64::MAX).fake())
        }
    }

    #[derive(Debug, Clone)]
    struct InValidValueF64(pub f64);

    impl quickcheck::Arbitrary for InValidValueF64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((f64::MIN..-f64::EPSILON).fake())
        }
    }

    #[quickcheck]
    fn a_negative_value_is_rejected(value: InValidValueF64) {
        assert_err!(ValidValue::parse(value.0));
    }

    #[test]
    fn a_zero_value_is_valid() {
        assert_ok!(ValidValue::parse(0.0));
    }

    #[quickcheck]
    fn a_valid_value_is_parsed_successfully(value: ValidValueF64) {
        assert_ok!(ValidValue::parse(value.0));
    }
}
