use serde::{Deserialize, Serialize};

#[derive(
    Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Debug, Hash, sqlx::Type,
)]
#[allow(non_camel_case_types)]
pub enum Series {
    #[serde(rename = "f1", alias = "F1")]
    f1,
    #[serde(rename = "f2", alias = "F2")]
    f2,
    #[serde(rename = "f3", alias = "F3")]
    f3,
}

impl From<Series> for String {
    fn from(value: Series) -> Self {
        match value {
            Series::f1 => "f1".to_owned(),
            Series::f2 => "f2".to_owned(),
            Series::f3 => "f3".to_owned(),
        }
    }
}

impl From<String> for Series {
    fn from(value: String) -> Self {
        return match value.as_str() {
            "f1" | "F1" => Series::f1,
            "f2" | "F2" => Series::f2,
            "f3" | "F3" => Series::f3,
            _ => Series::f1,
        };
    }
}

impl std::fmt::Display for Series {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        let str = match self {
            Self::f1 => "f1",
            Self::f2 => "f2",
            Self::f3 => "f3",
        };
        f.write_str(str)
    }
}
