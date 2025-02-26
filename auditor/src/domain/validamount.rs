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

// never turn this into `ValidAmount(pub i64)`. By keeping the inner field private, it is not
// possible to create this type outside of this module, hence enforcing the use of `parse`. This
// ensures that every string stored in this type satisfies the validation criteria checked by
// `parse`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, sqlx::Decode, sqlx::Encode)]
pub struct ValidAmount(i64);

impl ValidAmount {
    /// Returns `ValidAmount` only if input satisfies validation criteria, otherwise panics.
    pub fn parse(s: i64) -> Result<ValidAmount, ValidationError> {
        if s < 0 {
            Err(ValidationError(format!("Invalid amount: {s}")))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<i64> for ValidAmount {
    fn as_ref(&self) -> &i64 {
        &self.0
    }
}

impl Type<Postgres> for ValidAmount {
    fn type_info() -> PgTypeInfo {
        <&i64 as Type<Postgres>>::type_info()
    }

    fn compatible(ty: &PgTypeInfo) -> bool {
        <&i64 as Type<Postgres>>::compatible(ty)
    }
}

impl serde::Serialize for ValidAmount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_i64(self.0)
    }
}

impl<'de> serde::Deserialize<'de> for ValidAmount {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let buf = i64::deserialize(deserializer)?;
        ValidAmount::parse(buf)
            .with_context(|| format!("Parsing '{buf}' failed."))
            .map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for ValidAmount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::ValidAmount;
    use claim::{assert_err, assert_ok};
    use fake::Fake;

    #[derive(Debug, Clone)]
    struct ValidAmountI64(pub i64);

    impl quickcheck::Arbitrary for ValidAmountI64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((0..i64::MAX).fake())
        }
    }

    #[derive(Debug, Clone)]
    struct InValidAmountI64(pub i64);

    impl quickcheck::Arbitrary for InValidAmountI64 {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Self((i64::MIN..-1).fake())
        }
    }

    #[quickcheck_macros::quickcheck]
    fn a_negative_amount_is_rejected(amount: InValidAmountI64) {
        assert_err!(ValidAmount::parse(amount.0));
    }

    #[test]
    fn a_zero_amount_is_valid() {
        assert_ok!(ValidAmount::parse(0));
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_amount_is_parsed_successfully(amount: ValidAmountI64) {
        assert_ok!(ValidAmount::parse(amount.0));
    }
}
