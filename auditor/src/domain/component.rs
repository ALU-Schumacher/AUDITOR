// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use super::{Score, ScoreTest, ValidAmount, ValidName};
use anyhow::{Context, Error};
use fake::{Dummy, Fake, Faker, StringFaker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::{
    Postgres, Type,
    postgres::{PgHasArrayType, PgTypeInfo},
};

/// A `Component` represents a single component that is to be accounted for.
///
/// A component has an associated `name` and `amount` (how many or how much of this component is to
/// be accounted for).
/// Optionally, multiple [`Score`]s can be attached to a single component.
///
/// # Example:
///
/// Create a component that represents 10 CPU cores with a HEPSPEC06 value of 9.2.
///
/// ```
/// # use auditor::domain::{Component, Score};
/// # fn main() -> Result<(), anyhow::Error> {
/// let component = Component::new("CPU", 10)?
///     .with_score(Score::new("HEPSPEC06", 9.2)?);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, sqlx::Encode, Clone, PartialOrd, Ord)]
#[sqlx(type_name = "component")]
pub struct Component {
    /// Name of the component.
    pub name: ValidName,
    /// Amount of the component (how many or how much of this component is to be accounted for).
    pub amount: ValidAmount,
    /// Scores that are attached to the component.
    pub scores: Vec<Score>,
}

impl Component {
    /// Create a new component.
    ///
    /// # Errors
    ///
    /// * [`anyhow::Error`] - If there was an invalid character (`/()"<>\{}`) in the `name`
    ///   or if a negative `amount` was given.
    pub fn new<T: AsRef<str>>(name: T, amount: i64) -> Result<Self, Error> {
        Ok(Component {
            name: ValidName::parse(name.as_ref().to_string())
                .context("Failed to parse component name.")?,
            amount: ValidAmount::parse(amount).context("Failed to parse component amount.")?,
            scores: vec![],
        })
    }

    /// Attach a [`Score`] to the component.
    pub fn with_score(mut self, score: Score) -> Self {
        self.scores.push(score);
        self
    }

    /// Attach multiple [`Score`]s to the component.
    pub fn with_scores(mut self, mut scores: Vec<Score>) -> Self {
        self.scores.append(&mut scores);
        self
    }
}

// manual impl of decode because of a compiler bug. See:
// https://github.com/launchbadge/sqlx/issues/1031
// https://github.com/rust-lang/rust/issues/82219
impl sqlx::decode::Decode<'_, sqlx::Postgres> for Component {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, std::boxed::Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let name = decoder.try_decode::<ValidName>()?;
        let amount = decoder.try_decode::<ValidAmount>()?;
        let scores = decoder.try_decode::<Vec<Score>>()?;
        Ok(Component {
            name,
            amount,
            scores,
        })
    }
}

impl Type<Postgres> for Component {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("component")
    }
}

impl PgHasArrayType for Component {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_component")
    }
}

impl TryFrom<ComponentTest> for Component {
    type Error = Error;

    fn try_from(value: ComponentTest) -> Result<Self, Self::Error> {
        Ok(Component {
            name: ValidName::parse(value.name.ok_or_else(|| anyhow::anyhow!("name is None"))?)?,
            amount: ValidAmount::parse(
                value
                    .amount
                    .ok_or_else(|| anyhow::anyhow!("amount is None"))?,
            )?,
            scores: value
                .scores
                .into_iter()
                .map(Score::try_from)
                .collect::<Result<_, Self::Error>>()?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentTest {
    pub name: Option<String>,
    pub amount: Option<i64>,
    // Vecs can be empty, therefore no option needed
    pub scores: Vec<ScoreTest>,
}

impl PartialEq<Component> for ComponentTest {
    fn eq(&self, other: &Component) -> bool {
        let ComponentTest {
            name: s_name,
            amount: s_amount,
            scores: s_scores,
        } = self;
        let Component {
            name: o_name,
            amount: o_amount,
            scores: o_scores,
        } = other;

        // Can't be equal if any field in ComponentTest is None
        if s_name.is_none() || s_amount.is_none() {
            return false;
        }

        let mut s_scores = s_scores.clone();
        let mut o_scores = o_scores.clone();

        s_scores.sort();
        o_scores.sort();

        s_name.as_ref().unwrap() == o_name.as_ref()
            && s_amount.as_ref().unwrap() == o_amount.as_ref()
            && s_scores
                .into_iter()
                .zip(o_scores)
                .fold(true, |acc, (a, b)| acc && a == b)
    }
}

impl PartialEq<ComponentTest> for Component {
    fn eq(&self, other: &ComponentTest) -> bool {
        other.eq(self)
    }
}

impl Dummy<Faker> for ComponentTest {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> ComponentTest {
        let name = StringFaker::with(
            String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789*&^%$#@!~")
                .into_bytes(),
            1..256,
        )
        .fake_with_rng(rng);
        ComponentTest {
            name: Some(name),
            amount: Some((0..i64::MAX).fake_with_rng(rng)),
            scores: (0..(0..10u64).fake_with_rng(rng))
                .map(|_| Faker.fake_with_rng::<ScoreTest, _>(rng))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_ok;

    impl quickcheck::Arbitrary for ComponentTest {
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Faker.fake()
        }
    }

    #[quickcheck]
    fn a_valid_name_is_parsed_successfully(component: ComponentTest) {
        assert_ok!(Component::try_from(component));
    }
}
