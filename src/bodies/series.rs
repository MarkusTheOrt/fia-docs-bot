use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Series {
    kind: crate::model::series::Series,
    name: &'static str,
    data_source: &'static str,
}

impl Series {
    pub fn f1() -> Self {
        Self {
            kind: crate::model::series::Series::F1,
            name: "Formula 1",
            data_source: "https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2023-2042"
        }
    }

    pub fn f2() -> Self {
        Self {
            kind: crate::model::series::Series::F2,
            name: "Formula 2",
            data_source: "https://www.fia.com/documents/season/season-2023-2042/championships/formula-2-championship-44"
        }
    }

    pub fn f3() -> Self {
        Self { 
            kind: crate::model::series::Series::F3, 
            name: "FIA Formula 3", 
            data_source: "https://www.fia.com/documents/season/season-2023-2042/championships/fia-formula-3-championship-1012"
        }
    }
}
