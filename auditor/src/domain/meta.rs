// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::{cmp::Ordering, collections::HashMap};

use serde::{Deserialize, Serialize};

use super::ValidName;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
pub struct ValidMeta(pub HashMap<ValidName, Vec<ValidName>>);

impl ValidMeta {
    pub fn to_vec(&self) -> Vec<(String, Vec<String>)> {
        self.0
            .iter()
            .map(|(k, v)| {
                (
                    k.as_ref().to_string(),
                    v.iter().map(|v| v.as_ref().to_string()).collect(),
                )
            })
            .collect::<Vec<_>>()
    }
}

impl<T: AsRef<str>> TryFrom<HashMap<T, Vec<T>>> for ValidMeta {
    type Error = anyhow::Error;

    fn try_from(m: HashMap<T, Vec<T>>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|(k, v)| -> Result<_, Self::Error> {
                    Ok((
                        ValidName::parse(k.as_ref().to_string())?,
                        v.iter()
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

impl TryFrom<Vec<(String, Vec<String>)>> for ValidMeta {
    type Error = anyhow::Error;

    fn try_from(m: Vec<(String, Vec<String>)>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|um| -> Result<_, Self::Error> {
                    Ok((
                        ValidName::parse(um.0)?,
                        um.1.into_iter()
                            .map(|v| -> Result<_, Self::Error> { Ok(ValidName::parse(v)?) })
                            .collect::<Result<Vec<ValidName>, Self::Error>>()?,
                    ))
                })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

impl TryFrom<Meta> for ValidMeta {
    type Error = anyhow::Error;

    fn try_from(m: Meta) -> Result<Self, Self::Error> {
        Ok(Self(
            m.0.into_iter()
                .map(|(key, value)| -> Result<_, Self::Error> {
                    Ok((
                        ValidName::parse(key)?,
                        value
                            .into_iter()
                            .map(|v| -> Result<_, Self::Error> { Ok(ValidName::parse(v)?) })
                            .collect::<Result<Vec<ValidName>, Self::Error>>()?,
                    ))
                })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

/// `Meta` stores a list of key-value pairs of the form `String` -> `Vec<String>`.
///
/// # Example
///
/// Create a new meta list and insert two key-value pairs:
///
/// ```
/// # use auditor::domain::Meta;
/// #
/// let mut meta = Meta::new();
/// meta.insert("site_id".to_string(), vec!["site1".to_string()]);
/// meta.insert("features".to_string(), vec!["ssd".to_string(), "gpu".to_string()]);
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default, sqlx::FromRow)]
pub struct Meta(pub HashMap<String, Vec<String>>);

impl Meta {
    /// Constructor.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Returns the number of key-value pairs that are stored inside the meta list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if no information is stored in the meta list.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert to a vector.
    pub fn to_vec(&self) -> Vec<(String, Vec<String>)> {
        self.0
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>()
    }

    /// Insert a new key-value pair.
    pub fn insert(&mut self, name: String, values: Vec<String>) {
        self.0.insert(name, values);
    }

    /// Returns a reference to the value corresponding to the `key`.
    pub fn get<T: AsRef<str>>(&self, key: T) -> Option<&Vec<String>> {
        self.0.get(key.as_ref())
    }
}

impl From<ValidMeta> for Meta {
    fn from(m: ValidMeta) -> Self {
        Self(
            m.0.into_iter()
                .map(|(k, v)| {
                    (
                        k.as_ref().to_string(),
                        v.into_iter().map(|v| v.as_ref().to_string()).collect(),
                    )
                })
                .collect(),
        )
    }
}

impl<T: AsRef<str>> TryFrom<HashMap<T, Vec<T>>> for Meta {
    type Error = anyhow::Error;

    fn try_from(m: HashMap<T, Vec<T>>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|(k, v)| -> Result<_, Self::Error> {
                    Ok((
                        k.as_ref().to_string(),
                        v.into_iter()
                            .map(|v| -> Result<_, Self::Error> { Ok(v.as_ref().to_string()) })
                            .collect::<Result<Vec<String>, Self::Error>>()?,
                    ))
                })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

impl TryFrom<Vec<(String, Vec<String>)>> for Meta {
    type Error = anyhow::Error;

    fn try_from(m: Vec<(String, Vec<String>)>) -> Result<Self, Self::Error> {
        Ok(Self(
            m.into_iter()
                .map(|um| -> Result<_, Self::Error> { Ok((um.0.clone(), um.1)) })
                .collect::<Result<_, Self::Error>>()?,
        ))
    }
}

impl PartialOrd for Meta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Meta {
    fn cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}
