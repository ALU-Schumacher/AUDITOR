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

impl PartialEq<Component> for ComponentTest {
    fn eq(&self, other: &Component) -> bool {
        let ComponentTest {
            name: s_name,
            amount: s_amount,
            factor: s_factor,
        } = self;
        let Component {
            name: o_name,
            amount: o_amount,
            factor: o_factor,
        } = other;

        // Can't be equal if any field in ComponentTest is None
        if s_name.is_none() || s_amount.is_none() || s_factor.is_none() {
            return false;
        }

        let s_fac = f64::abs(*s_factor.as_ref().unwrap());
        let o_fac = f64::abs(*o_factor.as_ref());

        let (diff, biggest) = if s_fac > o_fac {
            (s_fac - o_fac, s_fac)
        } else {
            (o_fac - s_fac, o_fac)
        };

        s_name.as_ref().unwrap() == o_name.as_ref()
            && s_amount.as_ref().unwrap() == o_amount.as_ref()
            && (diff < f64::EPSILON || diff < biggest * f64::EPSILON)
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
        fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
            Faker.fake()
        }
    }

    #[quickcheck]
    fn a_valid_name_is_parsed_successfully(component: ComponentTest) {
        assert_ok!(Component::try_from(component));
    }
}
