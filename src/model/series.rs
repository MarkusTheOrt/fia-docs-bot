use serde::{Deserialize, Serialize};
use sqlx::{Decode, Encode, error::BoxDynError};


#[derive(Serialize, Deserialize, Clone, Copy, Eq, PartialEq, Debug)]
pub enum Series {
    #[serde(rename = "f1", alias = "F1")]
    F1,
    #[serde(rename = "f2", alias = "F2")]
    F2,
    #[serde(rename = "f3", alias = "F3")]
    F3,
}

impl Encode<'_, sqlx::MySql> for Series {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::MySql as sqlx::database::HasArguments<'_>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        let variant = match self {
            Self::F1 => "f1",
            Self::F2 => "f2",
            Self::F3 => "f3",
        };
        return variant.encode_by_ref(buf);
    }
}

impl<'r> Decode<'r, sqlx::MySql> for Series {
    fn decode(
        value: <sqlx::MySql as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, sqlx::error::BoxDynError> {
        let variant = String::decode(value)?;

        let series = match variant.as_str() {
            "f1" => Self::F1,
            "f2" => Self::F2,
            "f3" => Self::F3,
            _ => {
                return Err(
                    Box::new(
                        sqlx::Error::ColumnDecode { 
                            index: "racingseries".to_owned(), 
                            source: BoxDynError::from("test") 
                        })
                )
            }
        };

        return Ok(series);
    }
}
