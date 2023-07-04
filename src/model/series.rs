use serde::{Deserialize, Serialize};
use sqlx::{
    error::BoxDynError, mysql::MySqlTypeInfo, Decode, Encode, MySql, Type,
    TypeInfo, encode::IsNull, database::HasArguments,
};
use sqlx::types::chrono::DateTime;

#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Debug, Hash)]
pub enum Series {
    #[serde(rename = "f1", alias = "F1")]
    F1,
    #[serde(rename = "f2", alias = "F2")]
    F2,
    #[serde(rename = "f3", alias = "F3")]
    F3,
}

impl From<Series> for String {
    fn from(value: Series) -> Self {
        return match value {
            Series::F1 => "f1".to_owned(),
            Series::F2 => "f2".to_owned(),
            Series::F3 => "f3".to_owned(),
        };
    }
}

impl From<String> for Series {
    fn from(value: String) -> Self {
        return match value.as_str() {
            "f1" | "F1" => Series::F1,
            "f2" | "F2" => Series::F2,
            "f3" | "F3" => Series::F3,
            _ => Series::F1,
        }
    }
}

impl<'r> Decode<'r, sqlx::MySql> for Series {
    fn decode(
        value: <sqlx::MySql as sqlx::database::HasValueRef<'r>>::ValueRef
    ) -> Result<Self, BoxDynError> {
        let variant = String::decode(value)?;

        let series = match variant.as_str() {
            "f1" => Self::F1,
            "f2" => Self::F2,
            "f3" => Self::F3,
            _ => {
                return Err(Box::new(sqlx::error::Error::Decode(
                    "Error decoing Series".into(),
                )))
            },
        };
        return Ok(series);
    }
}

impl std::fmt::Display for Series {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let str = match self {
            Self::F1 => "f1",
            Self::F2 => "f2",
            Self::F3 => "f3",
        };
        return f.write_str(str);
    }
}

impl TypeInfo for Series {
    fn is_null(&self) -> bool {
        return false;
    }

    fn name(&self) -> &str {
        return "varchar(3)";
    }
}
