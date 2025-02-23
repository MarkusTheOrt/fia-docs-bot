use chrono::{DateTime, Utc};
use f1_bot_types::Series;



#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Guild {
    pub id: i64,
    pub discord_id: String,
    pub name: String,
    pub f1_role: Option<String>,
    pub f1_channel: Option<String>,
    pub f1_threads: bool,
    pub f2_role: Option<String>,
    pub f2_channel: Option<String>,
    pub f2_threads: bool,
    pub f3_role: Option<String>,
    pub f3_channel: Option<String>,
    pub f3_threads: bool,
    pub joined_at: DateTime<Utc>,
}

impl Guild {
    
    pub fn settings_for_series(&self, series: Series) -> (Option<&String>, Option<&String>, bool) {
        match series {
            Series::F1 => self.f1_settings(),
            Series::F2 => self.f2_settings(),
            Series::F3 => self.f3_settings(),
            Series::F1Academy => panic!("F1 Academy unsupported!"),
        }
    }

    pub fn f1_settings(&self) -> (Option<&String>, Option<&String>, bool) {
        (self.f1_role.as_ref(), self.f1_channel.as_ref(), self.f1_threads)
    } 
    pub fn f2_settings(&self) -> (Option<&String>, Option<&String>, bool) {
        (self.f2_role.as_ref(), self.f2_channel.as_ref(), self.f2_threads)
    } 
    pub fn f3_settings(&self) -> (Option<&String>, Option<&String>, bool) {
        (self.f3_role.as_ref(), self.f3_channel.as_ref(), self.f3_threads)
    } 
}
