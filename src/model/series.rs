use std::fmt::{self, Formatter};
use std::{io::Write, str::FromStr};

use serde::{Deserialize, Serialize};
use sqlx::decode::Decode;
use sqlx::{Encode, MySql};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RacingSeries {
    F1,
    F2,
    F3,
}

impl fmt::Display for RacingSeries {
    fn fmt(
        &self,
        f: &mut Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::F1 => write!(f, "F1"),
            Self::F2 => write!(f, "F2"),
            Self::F3 => write!(f, "F3"),
        }
    }
}

impl<'r> Decode<'r, sqlx::MySql> for RacingSeries {
    fn decode(
        value: <sqlx::MySql as sqlx::database::HasValueRef<'r>>::ValueRef
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let variant = <String as Decode<MySql>>::decode(value)?;
        let series: RacingSeries = variant.into();
        Ok(series)
    }
}

impl<'q> Encode<'q, MySql> for RacingSeries {
    fn encode_by_ref(
        &self,
        buf: &mut <MySql as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        if (match self {
            RacingSeries::F1 => buf.write_all(b"f1"),
            RacingSeries::F2 => buf.write_all(b"f2"),
            RacingSeries::F3 => buf.write_all(b"f3"),
        })
        .is_err()
        {
            return sqlx::encode::IsNull::Yes;
        }
        sqlx::encode::IsNull::No
    }
}

impl From<&String> for RacingSeries {
    fn from(value: &String) -> Self {
        match value.to_lowercase().as_str() {
            "f1" | "formula1" => Self::F1,
            "f2" | "formula2" => Self::F2,
            "f3" | "formula3" => Self::F3,
            _ => panic!("cannot parse this value."),
        }
    }
}

impl From<RacingSeries> for String {
    fn from(value: RacingSeries) -> Self {
        match value {
            RacingSeries::F1 => "f1".to_owned(),
            RacingSeries::F2 => "f2".to_owned(),
            RacingSeries::F3 => "f3".to_owned(),
        }
    }
}

impl From<String> for RacingSeries {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "f1" | "formula1" => Self::F1,
            "f2" | "formula2" => Self::F2,
            "f3" | "formula3" => Self::F3,
            _ => panic!("cannot parse this value."),
        }
    }
}

impl FromStr for RacingSeries {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "f1" | "formula1" => Ok(Self::F1),
            "f2" | "formula2" => Ok(Self::F2),
            "f3" | "formula3" => Ok(Self::F3),
            _ => Err("Not Found".to_owned()),
        }
    }
}
