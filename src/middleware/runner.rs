use super::{
    magick::{clear_tmp_dir, run_magick},
    parser::{HTMLParser, ParserEvent},
};
use crate::model::{event::Event, series::Series};
use aws_sign_v4::AwsSign;
use chrono::DateTime;
use html5ever::{
    tendril::{ByteTendril, ReadExt},
    tokenizer::{BufferQueue, Tokenizer, TokenizerOpts},
};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use sqlx::types::chrono::Utc;
use sqlx::{mysql::MySqlQueryResult, MySql, Pool};
use std::{
    error::Error, fs::File, num::NonZeroI16, path::PathBuf, str::FromStr,
    time::Duration,
};
use std::{
    io::{Read, Write},
    time::UNIX_EPOCH,
};

const F1_DOCS_URL:&str = "https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2024-2043";
const F2_DOCS_URL:&str = "https://www.fia.com/documents/season/season-2023-2042/championships/formula-2-championship-44";
const F3_DOCS_URL:&str = "https://www.fia.com/documents/season/season-2023-2042/championships/fia-formula-3-championship-1012";
const YEAR: i16 = 2023;

struct MinDoc {
    pub url: String,
}

struct LocalCache {
    pub documents: Vec<MinDoc>,
    pub events: Vec<Event>,
    pub last_populated: DateTime<Utc>,
}

impl Default for LocalCache {
    fn default() -> Self {
        Self {
            events: vec![],
            documents: vec![],
            last_populated: DateTime::from(UNIX_EPOCH),
        }
    }
}

async fn populate_cache(
    pool: &Pool<MySql>,
    cache: &mut LocalCache,
    series: Series,
) {
    let delta = Utc::now() - cache.last_populated;
    // lets revalidate the cache once a day.
    if delta.num_days() < 1 {
        return;
    }
    let series_str: String = series.into();
    let docs: Vec<MinDoc> = match sqlx::query_as!(
        MinDoc,
        r#"
    SELECT url
    FROM documents
    WHERE series = ? AND YEAR(created) = ?"#,
        series_str,
        YEAR
    )
    .fetch_all(pool)
    .await
    {
        Ok(data) => data,
        Err(why) => {
            eprintln!("Error populating cache: {why}");
            return;
        },
    };

    let events: Vec<Event> = match sqlx::query_as_unchecked!(
        Event,
        r#"SELECT 
        `id` as `id?`, 
        year, 
        series, 
        name, 
        created 
        FROM
        events where year = ? AND 
        series = ?"#,
        YEAR,
        series_str
    )
    .fetch_all(pool)
    .await
    {
        Ok(data) => data,
        Err(why) => {
            eprintln!("Error populating events: {why}");
            return;
        },
    };
    cache.events = events;
    cache.documents = docs;
    cache.last_populated = Utc::now();
    println!("Repopulated cache!");
    println!(
        "{series} events: {}, docs: {}",
        cache.events.len(),
        cache.documents.len()
    );
}

pub async fn runner(pool: &Pool<MySql>) {
    let mut f1_local_cache = LocalCache::default();
    let mut f2_local_cache = LocalCache::default();
    let mut f3_local_cache = LocalCache::default();

    loop {
        let start = Utc::now();
        println!("Scanning for documents.");
        populate_cache(pool, &mut f1_local_cache, Series::F1).await;
        populate_cache(pool, &mut f2_local_cache, Series::F2).await;
        populate_cache(pool, &mut f3_local_cache, Series::F3).await;

        #[cfg(not(debug_assertions))]
        {
            f1_runner(pool, YEAR, F1_DOCS_URL, Series::F1, &mut f1_local_cache)
                .await;
            //           f1_runner(pool, YEAR, F2_DOCS_URL, Series::F2, &mut f2_local_cache)
            //               .await;
            //           f1_runner(pool, YEAR, F3_DOCS_URL, Series::F3, &mut f3_local_cache)
            //               .await;
        }
        let runner_time = (Utc::now() - start).to_std().unwrap();

        std::thread::sleep(
            Duration::from_secs(180)
                .checked_sub(runner_time)
                .unwrap_or(Duration::from_secs(1)),
        );
    }
}

async fn f1_runner(
    pool: &Pool<MySql>,
    year: i16,
    url: &str,
    series: Series,
    cache: &mut LocalCache,
) {
    let season = match get_season(url, NonZeroI16::new(year).unwrap()).await {
        Ok(season) => season,
        Err(why) => {
            eprintln!("Error fetching: {why}");
            return;
        },
    };
    let series_str: String = series.into();
    for ev in season.events {
        let year: i16 = season.year.into();
        let cache_event = cache.events.iter().find(|f| {
            ev.title.as_ref().is_some_and(|t| *t == f.name)
                && ev.season.is_some_and(|s| i16::from(s) == f.year as i16)
        });

        let db_event: Event = match cache_event {
            Some(db_event) => db_event.clone(),
            None => match sqlx::query_as_unchecked!(
                Event,
                "SELECT `id` as `id?`, name, year, created, series FROM events where name = ? AND year = ? AND series = ?",
                ev.title,
                year,
                series_str
            )
                .fetch_optional(pool)
                .await {
                Ok(Some(db_event)) => {
                        cache.events.push(db_event.clone());
                        db_event
                    },
                Ok(None) => {
                    match insert_event(pool, year, &ev, series).await {
                        Err(why) => {
                            eprintln!("Error creating event: {why}");
                            return;
                        },
                        Ok(event) => {
                                cache.events.push(event.clone());
                                event
                            }
                    }
                },
                Err(why) => {
                    eprintln!("sqlx Error: {why}");
                    continue;
                }
            }
        };
        for (i, doc) in ev.documents.iter().enumerate() {
            if let Some(_) = cache.documents.iter().find(|f| {
                return f.url == *doc.url.as_ref().unwrap();
            }) {
                continue;
            }
            println!("doc not found!");
            let (title, url, _) = (
                doc.title.as_ref().unwrap(),
                doc.url.as_ref().unwrap(),
                doc.date.as_ref().unwrap(),
            );
            let (file, body) =
                match download_file(url, &format!("doc_{i}")).await {
                    Err(why) => {
                        eprintln!("Download Error: {why}");
                        continue;
                    },
                    Ok(data) => data,
                };

            let mirror_url =
                match upload_mirror(title, &db_event.name, year, &body).await {
                    Err(why) => {
                        eprintln!("error uploading mirror doc:{why}");
                        continue;
                    },
                    Ok(url) => url,
                };

            let series_str: String = series.into();
            let inserted_doc: MySqlQueryResult = match sqlx::query_unchecked!(
                "INSERT INTO documents (event, url, title, series, mirror) VALUES (?, ?, ?, ?, ?)",
                    db_event.id.as_ref().unwrap(),
                    url,
                    title,
                    series_str,
                    mirror_url
                ).execute(pool).await {
                        Err(why) => {
                            eprintln!("Error inserting doc: {why}");
                            continue;
                        }
                        Ok(data) => data
                    };
            println!("adding doc {title}");
            cache.documents.push(MinDoc {
                url: url.clone(),
            });
            let files =
                match run_magick(file.to_str().unwrap(), &format!("doc_{i}")) {
                    Err(why) => {
                        eprintln!("error running magick: {why}");
                        continue;
                    },
                    Ok(data) => data,
                };

            for (j, path) in files.iter().enumerate() {
                let mut file = match File::open(path) {
                    Err(why) => {
                        eprintln!("Error opening file: {why}");
                        continue;
                    },
                    Ok(data) => data,
                };

                // I think 10 Mb is a reasonable size, most docs will be under that.
                let mut buf = Vec::with_capacity(1024 * 1024 * 10);
                match file.read_to_end(&mut buf) {
                    Err(why) => {
                        eprintln!("Error reading file: {why}");
                        continue;
                    },
                    Ok(data) => data,
                };
                let digest = sha256::digest(buf.as_slice());

                let url = format!(
                    "https://fia.ort.dev/{}/{}/{}-{}.jpg",
                    year,
                    urlencoding::encode(ev.title.as_ref().unwrap()),
                    inserted_doc.last_insert_id(),
                    j
                );
                let now = Utc::now();
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert("x-amz-content-sha256", digest.parse().unwrap());
                headers.insert("x-amz-acl", "public-read".parse().unwrap());
                headers.insert(
                    "X-Amz-Date",
                    now.format("%Y%m%dT%H%M%SZ").to_string().parse().unwrap(),
                );
                headers.insert("host", "fia.ort.dev".parse().unwrap());
                let secret = std::env::var("S3_SECRET_KEY").unwrap();
                let access = std::env::var("S3_ACCESS_KEY").unwrap();
                let sign = AwsSign::new(
                    "PUT",
                    &url,
                    &now,
                    &headers,
                    "us-east-1",
                    &access,
                    &secret,
                    "s3",
                    Some(&digest),
                );
                let signature = sign.sign();
                headers.insert(AUTHORIZATION, signature.parse().unwrap());
                headers.insert(CONTENT_TYPE, "image/jpeg".parse().unwrap());
                let client = reqwest::Client::new();
                match client.put(&url).headers(headers).body(buf).send().await {
                    Ok(data) => match data.error_for_status() {
                        Err(why) => {
                            eprintln!("Uploade Error: {why}");
                        },
                        Ok(_) => {
                            match insert_image(
                                inserted_doc.last_insert_id(),
                                j as u32,
                                url,
                                pool,
                            )
                            .await
                            {
                                Err(why) => eprintln!("Error inserting: {why}"),
                                Ok(_) => {},
                            }
                        },
                    },
                    Err(why) => {
                        eprintln!("Error: {why}");
                    },
                }
            }
            match mark_doc_done(inserted_doc.last_insert_id(), pool).await {
                Ok(_) => {},
                Err(why) => {
                    println!("Error marking doc done: {why}");
                },
            }
        }
        if let Err(why) = clear_tmp_dir() {
            eprintln!("couldn't clear temp dir: {why}");
        }
    }
}

async fn mark_doc_done(
    doc_id: u64,
    pool: &Pool<MySql>,
) -> Result<(), Box<dyn Error>> {
    sqlx::query!("UPDATE documents SET done = 1 WHERE id = ?", doc_id)
        .execute(pool)
        .await?;

    return Ok(());
}

async fn insert_image(
    doc_id: u64,
    page: u32,
    url: String,
    pool: &Pool<MySql>,
) -> Result<(), Box<dyn Error>> {
    sqlx::query!(
        "INSERT INTO images (document, url, pagenum) VALUES (?, ?, ?)",
        doc_id,
        url,
        page
    )
    .execute(pool)
    .await?;

    return Ok(());
}

async fn upload_mirror(
    title: &str,
    event: &str,
    year: i16,
    content: &Vec<u8>,
) -> Result<String, Box<dyn Error>> {
    let now = Utc::now();
    let title = urlencoding::encode(title);
    let url = format!("https://fia.ort.dev/mirror/{year}/{event}/{title}.pdf");
    let digest = sha256::digest(content.as_slice());
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-amz-content-sha256", digest.parse().unwrap());
    headers.insert("x-amz-acl", "public-read".parse().unwrap());
    headers.insert(
        "X-Amz-Date",
        now.format("%Y%m%dT%H%M%SZ").to_string().parse().unwrap(),
    );
    headers.insert("host", "fia.ort.dev".parse().unwrap());
    let secret = std::env::var("S3_SECRET_KEY").unwrap();
    let access = std::env::var("S3_ACCESS_KEY").unwrap();
    let sign = AwsSign::new(
        "PUT",
        &url,
        &now,
        &headers,
        "us-east-1",
        &access,
        &secret,
        "s3",
        Some(&digest),
    );
    let signature = sign.sign();
    headers.insert(AUTHORIZATION, signature.parse().unwrap());
    headers.insert(CONTENT_TYPE, "application/pdf".parse().unwrap());

    let client = reqwest::Client::new();
    let t = client
        .put(url)
        .headers(headers)
        .body(content.to_owned())
        .send()
        .await?;
    let url = t.url().to_string();
    t.error_for_status()?;
    return Ok(url);
}

async fn download_file(
    url: &str,
    name: &str,
) -> Result<(PathBuf, Vec<u8>), Box<dyn Error>> {
    let request = reqwest::get(url).await?;
    let mut file = File::create(format!("./tmp/{name}.pdf"))?;
    let body = request.bytes().await?;
    file.set_len(body.len() as u64)?;
    file.write_all(&body)?;
    let path = PathBuf::from_str(&format!("./tmp/{name}.pdf"))?;
    // ensure we're actually pointing to a legit file.
    path.try_exists()?;
    return Ok((path, body.to_vec()));
}

async fn insert_event(
    pool: &Pool<MySql>,
    year: i16,
    event: &ParserEvent,
    series: Series,
) -> Result<Event, Box<dyn Error>> {
    let mut db_event = Event {
        id: None,
        series,
        year: year as u32,
        name: event.title.as_ref().unwrap().clone(),
        created: Utc::now(),
    };
    let series: String = db_event.series.into();
    let res: MySqlQueryResult = sqlx::query_unchecked!("INSERT INTO events (series, year, name, created, current, new) VALUES (?, ?, ?, ?, 0, 1)",
    series,
    db_event.year,
    db_event.name,
    db_event.created).execute(pool).await?;
    db_event.id = Some(res.last_insert_id());
    return Ok(db_event);
}

async fn get_season(
    url: &str,
    year: NonZeroI16,
) -> Result<super::parser::Season, Box<dyn Error>> {
    let test = reqwest::get(url).await?;

    let bytes = test.text().await?;

    let mut tendril = ByteTendril::new();
    let _ = bytes.as_bytes().read_to_tendril(&mut tendril);
    let mut input = BufferQueue::new();
    input.push_back(tendril.try_reinterpret().unwrap());
    let mut parser_season = super::parser::Season {
        year,
        events: vec![],
    };
    let sink = HTMLParser::new(&mut parser_season);
    let opts = TokenizerOpts::default();
    let mut tok = Tokenizer::new(sink, opts);
    let _ = tok.feed(&mut input);
    tok.end();
    return Ok(parser_season);
}
