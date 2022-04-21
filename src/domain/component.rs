use super::{ValidAmount, ValidFactor, ValidName};
use fake::{Dummy, Fake, Faker, StringFaker};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgHasArrayType;

#[derive(Debug, PartialEq, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "component")]
pub struct Component {
    pub name: ValidName,
    pub amount: ValidAmount,
    pub factor: ValidFactor,
}

impl PgHasArrayType for Component {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_component")
    }
}

impl TryFrom<ComponentTest> for Component {
    type Error = String;

    fn try_from(value: ComponentTest) -> Result<Self, Self::Error> {
        Ok(Component {
            name: ValidName::parse(value.name.ok_or_else(|| "name is None".to_string())?)?,
            amount: ValidAmount::parse(value.amount.ok_or_else(|| "amount is None".to_string())?)?,
            factor: ValidFactor::parse(value.factor.ok_or_else(|| "factor is None".to_string())?)?,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentTest {
    pub name: Option<String>,
    pub amount: Option<i64>,
    pub factor: Option<f64>,
}

impl Dummy<Faker> for ComponentTest {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> ComponentTest {
        let name = StringFaker::with(
            String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789*&^%$#@!~").into_bytes(),
            1..256,
        )
        .fake_with_rng(rng);
        ComponentTest {
            name: Some(name),
            amount: Some((0..i64::MAX).fake_with_rng(rng)),
            factor: Some((0.0..f64::MAX).fake_with_rng(rng)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claim::assert_ok;

    impl quickcheck::Arbitrary for ComponentTest {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            Faker.fake_with_rng(g)
        }
    }

    #[quickcheck_macros::quickcheck]
    fn a_valid_name_is_parsed_successfully(component: ComponentTest) {
        assert_ok!(Component::try_from(component));
    }
}
