// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::{
    postgres::{PgHasArrayType, PgTypeInfo},
    Postgres, Type,
};

use super::ValidName;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct Meta(pub HashMap<ValidName, Vec<ValidName>>);

impl Meta {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn to_vec(&self) -> Vec<(String, Vec<String>)> {
        self.0
            .iter()
            .map(|(k, v)| {
                (
                    k.as_ref().to_string(),
                    v.iter().map(|v| v.as_ref().to_string()).collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn to_vec_unit(&self) -> Vec<UnitMeta> {
        self.0
            .iter()
            .map(|(k, v)| UnitMeta {
                key: k.to_string(),
                value: v.iter().map(|s| s.to_string()).collect(),
            })
            .collect::<Vec<_>>()
    }
}

impl<T: AsRef<str>> TryFrom<HashMap<T, Vec<T>>> for Meta {
    type Error = anyhow::Error;

    fn try_from(m: HashMap<T, Vec<T>>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|(k, v)| -> Result<_, Self::Error> {
                    Ok((
                        ValidName::parse(k.as_ref().to_string())?,
                        v.into_iter()
                            .map(|v| -> Result<_, Self::Error> {
                                Ok(ValidName::parse(v.as_ref().to_string())?)
                            })
                            .collect::<Result<Vec<ValidName>, Self::Error>>()?,
                    ))
                })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

impl TryFrom<Vec<UnitMeta>> for Meta {
    type Error = anyhow::Error;

    fn try_from(m: Vec<UnitMeta>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|um| -> Result<_, Self::Error> {
                    Ok((
                        ValidName::parse(um.key)?,
                        um.value
                            .into_iter()
                            .map(|v| -> Result<_, Self::Error> { Ok(ValidName::parse(v)?) })
                            .collect::<Result<Vec<ValidName>, Self::Error>>()?,
                    ))
                })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

#[derive(
    Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default, sqlx::Encode, PartialOrd, Ord,
)]
#[sqlx(type_name = "unit_meta")]
pub struct UnitMeta {
    key: String,
    value: Vec<String>,
}

impl<T: AsRef<str>> From<(T, Vec<T>)> for UnitMeta {
    fn from(m: (T, Vec<T>)) -> Self {
        Self {
            key: m.0.as_ref().to_string(),
            value: m.1.iter().map(|v| v.as_ref().to_string()).collect(),
        }
    }
}

// impl<'q> Encode<'q, Postgres> for (String, Vec<String>) {
//     #[inline]
//     fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
//         buf.extend(self.0.as_bytes());
//         buf.extend(self.1.as_bytes());
//         IsNull::No
//     }
// }

// impl<'q> Encode<'q, Postgres> for Meta
// where
//     for<'a> &'a [(String, Vec<String>)]: Encode<'q, Postgres>,
//     // Vec<(&'q String, &'q Vec<String>)>: Encode<'q, Postgres>,
//     (String, Vec<String>): Encode<'q, Postgres>,
//     String: Encode<'q, Postgres>,
//     // (ValidName, Vec<ValidName>): Encode<'q, Postgres>,
//     // ValidName: Encode<'q, Postgres>,
// {
//     #[inline]
//     fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> IsNull {
//         self.to_vec().encode_by_ref(buf)
//     }
// }

// // manual impl of decode because of a compiler bug. See:
// // https://github.com/launchbadge/sqlx/issues/1031
// // https://github.com/rust-lang/rust/issues/82219
// impl sqlx::decode::Decode<'_, sqlx::Postgres> for HashMapDatabase {
//     fn decode(
//         value: sqlx::postgres::PgValueRef<'_>,
//     ) -> Result<Self, std::boxed::Box<dyn std::error::Error + 'static + Send + Sync>> {
//         let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
//         let meta = decoder
//             .try_decode::<Vec<HashMapDatabase>>()?
//             .into_iter()
//             .collect::<HashMap<String, Vec<String>>>()
//             .try_into()?;
//         Ok(meta)
//     }
// }

// manual impl of decode because of a compiler bug. See:
// https://github.com/launchbadge/sqlx/issues/1031
// https://github.com/rust-lang/rust/issues/82219
impl sqlx::decode::Decode<'_, sqlx::Postgres> for UnitMeta {
    fn decode(
        value: sqlx::postgres::PgValueRef<'_>,
    ) -> Result<Self, std::boxed::Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;
        let key = decoder.try_decode::<String>()?;
        let value = decoder.try_decode::<Vec<String>>()?;
        Ok(UnitMeta { key, value })
    }
}

impl Type<Postgres> for UnitMeta {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_unit_meta")
    }
}

impl PgHasArrayType for UnitMeta {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_unit_meta")
    }
}
