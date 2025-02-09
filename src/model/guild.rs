use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
pub struct DbGuild {
    pub id: u64,
    pub name: String,
    pub channel: Option<u64>,
    pub notify_role: Option<u64>,
    pub joined: DateTime<Utc>,
}

//pub async fn insert_new_guild(
//    guild: &Guild,
//    pool: &Pool<MySql>,
//    guild_cache: &Arc<Mutex<GuildCache>>,
//) -> Result<sqlx::mysql::MySqlQueryResult, sqlx::Error> {
//    let new_guild = DbGuild {
//        id: guild.id.get(),
//        name: guild.name.clone(),
//        channel: None,
//        notify_role: None,
//        joined: Utc::now(),
//    };
//
//    {
//        let mut cache = guild_cache.lock().unwrap();
//        match cache.cache.iter_mut().find(|p| p.id == guild.id.get()) {
//            Some(_) => {},
//            None => {
//                cache.cache.push(CachedGuild {
//                    id: guild.id.get(),
//                    f1: SeriesSettings::default(),
//                    f2: SeriesSettings::default(),
//                    f3: SeriesSettings::default(),
//                });
//            },
//        }
//    }
//
//    sqlx::query!(
//        "INSERT INTO guilds(id, name, joined) VALUES (?, ?, ?) ON DUPLICATE KEY update name = ?",
//        new_guild.id,
//        new_guild.name,
//        new_guild.joined,
//        new_guild.name
//    )
//    .execute(pool)
//    .await
//}

