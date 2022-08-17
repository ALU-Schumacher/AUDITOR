// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use crate::domain::ValidationError;
use anyhow::Context;
use sqlx::{postgres::PgTypeInfo, Postgres, Type};
use std::fmt;

// never turn this into `ValidFactor(pub f64)`. By keeping the inner field private, it is not
// possible to create this type outside of this module, hence enforcing the use of `parse`. This
// ensures that every string stored in this type satisfies the validation criteria checked by
// `parse`.
#[derive(Debug, Clone, Copy, PartialEq, sqlx::Decode, sqlx::Encode)]
pub struct ValidFactor(f64);

impl ValidFactor {
    /// Returns `ValidFactor` only if input satisfies validation criteria, otherwise panics.
    pub fn parse(s: f64) -> Result<ValidFactor, ValidationError> {
        if s < 0.0 {
            Err(ValidationError(format!("Invalid factor: {}", s)))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<f64> for ValidFactor {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl Type<Postgres> for ValidFactor {
    fn type_info() -> PgTypeInfo {
        <&f64 as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&f64 as Type<Postgres>>::compatible(ty)
    }
}

impl serde::Serialize for ValidFactor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ValidFactor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf = f64::deserialize(deserializer)?;
        ValidFactor::parse(buf)
            .with_context(|| format!("Parsing '{}' failed.", buf))
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ValidFactor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::ValidFactor;
    use claim::{assert_err, assert_ok};
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidFactorF64(pub f64);

    impl quickcheck::Arbitrary for ValidFactorF64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((0.0..f64::MAX).fake())
        }
    }

    #[derive(Debug, Clone)]
    struct InValidFactorF64(pub f64);

    impl quickcheck::Arbitrary for InValidFactorF64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((f64::MIN..-f64::EPSILON).fake())
        }
    }

    #[quickcheck]
    fn a_negative_factor_is_rejected(factor: InValidFactorF64) {
        assert_err!(ValidFactor::parse(factor.0));
    }

    #[test]
    fn a_zero_factor_is_valid() {
        assert_ok!(ValidFactor::parse(0.0));
    }

    #[quickcheck]
    fn a_valid_factor_is_parsed_successfully(factor: ValidFactorF64) {
        assert_ok!(ValidFactor::parse(factor.0));
    }
}
