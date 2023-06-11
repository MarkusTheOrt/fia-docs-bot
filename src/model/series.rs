use std::str::FromStr;

use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RacingSeries {
    F1,
    F2,
    F3,
    WRX,
    WRC
}
impl From<&String> for RacingSeries {
    fn from(value: &String) -> Self {
        match value.to_lowercase().as_str() {
            "f1" | "formula1" => return Self::F1,
            "f2" | "formula2" => return Self::F2,
            "f3" | "formula3" => return Self::F3,
            "wrx" | "world rally cross" => return Self::WRX,
            "wrc" | "world rally championship" => return Self::WRC,
            _ => panic!("cannot parse this value.")
        }
    }
}

impl From<String> for RacingSeries {
    fn from(value: String) -> Self {
        match value.to_lowercase().as_str() {
            "f1" | "formula1" => return Self::F1,
            "f2" | "formula2" => return Self::F2,
            "f3" | "formula3" => return Self::F3,
            "wrx" | "world rally cross" => return Self::WRX,
            "wrc" | "world rally championship" => return Self::WRC,
            _ => panic!("cannot parse this value.")
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
            "wrx" | "world rally cross" => Ok(Self::WRX),
            "wrc" | "world rally championship" => Ok(Self::WRC),
            _ => Err("Not Found".to_owned())
        }
    }
}

/// This struct represents a racing series (F1, F2, F3, WRC, etc...)
/// NOTE: THIS IS CURRENTLY NOT IN USE!
#[derive(Serialize, Deserialize)]
pub struct Series {
    pub id: u64,
    pub name: String,
    pub short_handle: String,
}
