use super::{
    magick::{clear_tmp_dir, run_magick},
    parser::{HTMLParser, ParserEvent},
};
use aws_sign_v4::AwsSign;
use chrono::{DateTime, Datelike, Utc};
use f1_bot_types::{Document, Event, Series};
use html5ever::{
    tendril::{fmt::Slice, ByteTendril, ReadExt, SliceExt},
    tokenizer::{BufferQueue, Tokenizer, TokenizerOpts},
};
use libsql::{params, Connection};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use std::{
    cell::RefCell, fs::File, path::PathBuf, str::FromStr, time::Duration,
};
use std::{
    io::{Read, Write},
    time::UNIX_EPOCH,
};
use tracing::{error, info};

const F1_DOCS_URL:&str = "https://www.fia.com/documents/championships/fia-formula-one-world-championship-14/season/season-2024-2043";
const F2_DOCS_URL:&str = "https://www.fia.com/documents/season/season-2024-2043/championships/formula-2-championship-44";
const F3_DOCS_URL:&str = "https://www.fia.com/documents/season/season-2024-2043/championships/fia-formula-3-championship-1012";

struct LocalCache {
    pub documents: Vec<Document>,
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
    db_conn: &Connection,
    cache: &mut LocalCache,
    series: Series,
    year: i32,
) -> crate::error::Result {
    let delta = Utc::now() - cache.last_populated;
    // lets revalidate the cache once a day.
    if delta.num_days() < 1 {
        return Ok(());
    }
    let mut docs = db_conn.query("SELECT * FROM documents WHERE series = ? and strftime('%Y', created) = ?", params![series, year],).await?;
    let mut events = db_conn
        .query(
            "SELECT * FROM events WHERE year = ? AND series = ?",
            params![year, series],
        )
        .await?;
    cache.documents.clear();
    while let Ok(Some(doc)) = docs.next().await {
        cache.documents.push(libsql::de::from_row(&doc)?);
    }
    while let Ok(Some(event)) = events.next().await {
        cache.events.push(libsql::de::from_row(&event)?);
    }

    cache.last_populated = Utc::now();
    Ok(())
}

pub async fn runner(db_conn: Connection) -> crate::error::Result {
    let mut f1_local_cache = LocalCache::default();
    let mut f2_local_cache = LocalCache::default();
    let mut f3_local_cache = LocalCache::default();

    loop {
        let start = Utc::now();
        info!("Scanning for documents.");
        let year = start.year();
        populate_cache(&db_conn, &mut f1_local_cache, Series::F1, year).await?;
        populate_cache(&db_conn, &mut f2_local_cache, Series::F2, year).await?;
        populate_cache(&db_conn, &mut f3_local_cache, Series::F3, year).await?;

        {
            runner_internal(
                &db_conn,
                year,
                F1_DOCS_URL,
                Series::F1,
                &mut f1_local_cache,
            )
            .await;
            runner_internal(
                &db_conn,
                year,
                F2_DOCS_URL,
                Series::F2,
                &mut f2_local_cache,
            )
            .await;
            runner_internal(
                &db_conn,
                year,
                F3_DOCS_URL,
                Series::F3,
                &mut f3_local_cache,
            )
            .await;
        }
        let runner_time = (Utc::now() - start).to_std().unwrap();

        tokio::time::sleep(
            Duration::from_secs(180)
                .checked_sub(runner_time)
                .unwrap_or(Duration::from_secs(1)),
        )
        .await;
    }
}

async fn runner_internal(
    db_conn: &Connection,
    year: i32,
    url: &str,
    series: Series,
    cache: &mut LocalCache,
) -> crate::error::Result {
    let season = get_season(url, year).await?;
    let series_str: String = series.into();
    for ev in season.events {
        let cache_event = cache.events.iter().find(|f| {
            ev.title.as_ref().is_some_and(|t| *t == f.title)
                && ev.season.is_some_and(|s| s == year)
        });

        // let db_event: &Event = match cache_event.as_ref() {
        //     Some(db_event) => db_event,
        //     None => match sqlx::query_as_unchecked!(
        //         Event,
        //         "SELECT id as \"id?\", name, year, created, series FROM events where name = $1 AND year = $2 AND series = $3",
        //         ev.title,
        //         year,
        //         series_str
        //     )
        //         .fetch_optional(db_conn)
        //         .await {
        //         Ok(Some(db_event)) => {
        //                 cache.events.push(db_event.clone());
        //                 db_event
        //             },
        //         Ok(None) => {
        //             cache.events.push(insert_event(db_conn, year, &ev, series).await?);
        //         },
        //         Err(why) => {
        //             error!("sqlx Error: {why}");
        //             continue;
        // title            }
        // };
        // for (i, doc) in ev.documents.iter().enumerate() {
        //     if cache.documents.iter().any(|f| {
        //         return f.url == *doc.url.as_ref().unwrap();
        //     }) {
        //         continue;
        //     }
        //     println!("doc not found!");
        //     let (title, url, _) = (
        //         doc.title.as_ref().unwrap(),
        //         doc.url.as_ref().unwrap(),
        //         doc.date.as_ref().unwrap(),
        //     );
        //     let (file, body) =
        //         match download_file(url, &format!("doc_{i}")).await {
        //             Err(why) => {
        //                 eprintln!("Download Error: {why}");
        //                 continue;
        //             },
        //             Ok(data) => data,
        //         };

        //     let mirror_url =
        //         match upload_mirror(title, &db_event.name, year, &body).await {
        //             Err(why) => {
        //                 eprintln!("error uploading mirror doc:{why}");
        //                 continue;
        //             },
        //             Ok(url) => url,
        //         };

        //     let series_str: String = series.into();
        //     struct Id {
        //         id: i64,
        //     }
        //     let inserted_doc: Id = match sqlx::query_as_unchecked!(Id,
        //         "INSERT INTO documents (event, url, title, series, mirror) VALUES ($1, $2, $3, $4, $5) RETURNING id",
        //             db_event.id.as_ref().unwrap(),
        //             url,
        //             title,
        //             series_str,
        //             mirror_url
        //         ).fetch_one(db_conn).await {
        //                 Err(why) => {
        //                     eprintln!("Error inserting doc: {why}");
        //                     continue;
        //                 }
        //                 Ok(data) => data
        //             };
        //     println!("adding doc {title}");
        //     let files =
        //         match run_magick(file.to_str().unwrap(), &format!("doc_{i}")) {
        //             Err(why) => {
        //                 eprintln!("error running magick: {why}");
        //                 continue;
        //             },
        //             Ok(data) => data,
        //         };

        //     for (j, path) in files.iter().enumerate() {
        //         let mut file = match File::open(path) {
        //             Err(why) => {
        //                 eprintln!("Error opening file: {why}");
        //                 continue;
        //             },
        //             Ok(data) => data,
        //         };

        //         // I think 10 Mb is a reasonable size, most docs will be under that.
        //         let mut buf = Vec::with_capacity(1024 * 1024 * 10);
        //         match file.read_to_end(&mut buf) {
        //             Err(why) => {
        //                 eprintln!("Error reading file: {why}");
        //                 continue;
        //             },
        //             Ok(data) => data,
        //         };
        //         let digest = sha256::digest(buf.as_slice());

        //         let url = format!(
        //             "https://fia.ort.dev/{}/{}/{}-{}.jpg",
        //             year,
        //             urlencoding::encode(ev.title.as_ref().unwrap()),
        //             inserted_doc.id,
        //             j
        //         );
        //         let now = Utc::now();
        //         let mut headers = reqwest::header::HeaderMap::new();
        //         headers.insert("x-amz-content-sha256", digest.parse().unwrap());
        //         headers.insert("x-amz-acl", "public-read".parse().unwrap());
        //         headers.insert(
        //             "X-Amz-Date",
        //             now.format("%Y%m%dT%H%M%SZ").to_string().parse().unwrap(),
        //         );
        //         headers.insert("host", "fia.ort.dev".parse().unwrap());
        //         let secret = std::env::var("S3_SECRET_KEY").unwrap();
        //         let access = std::env::var("S3_ACCESS_KEY").unwrap();
        //         let sign = AwsSign::new(
        //             "PUT",
        //             &url,
        //             &now,
        //             &headers,
        //             "us-east-1",
        //             &access,
        //             &secret,
        //             "s3",
        //             Some(&digest),
        //         );
        //         let signature = sign.sign();
        //         headers.insert(AUTHORIZATION, signature.parse().unwrap());
        //         headers.insert(CONTENT_TYPE, "image/jpeg".parse().unwrap());
        //         let client = reqwest::Client::new();
        //         match client.put(&url).headers(headers).body(buf).send().await {
        //             Ok(data) => match data.error_for_status() {
        //                 Err(why) => {
        //                     eprintln!("Uploade Error: {why}");
        //                 },
        //                 Ok(_) => {
        //                     if let Err(why) = insert_image(
        //                         inserted_doc.id,
        //                         j as i32,
        //                         url,
        //                         db_conn,
        //                     )
        //                     .await
        //                     {
        //                         eprintln!("Error inserting: {why}")
        //                     }
        //                 },
        //             },
        //             Err(why) => {
        //                 eprintln!("Error: {why}");
        //             },
        //         }
        //     }
        //     _ = mark_doc_done(inserted_doc.id, db_conn).await?;
        // }
        // if let Err(why) = clear_tmp_dir() {
        //     eprintln!("couldn't clear temp dir: {why}");
        // }
    }
    Ok(())
}

async fn mark_doc_done(
    doc_id: i64,
    db_conn: &Connection,
) -> crate::error::Result {
    // let id = sqlx::query_as!(
    //     Id,
    //     "UPDATE documents SET done = 1 WHERE id = $1 RETURNING id",
    //     doc_id
    // )
    // .fetch_one(pool)
    // .await?;
    Ok(())
}

async fn insert_image(
    doc_id: i64,
    page: i32,
    url: String,
    pool: &Connection,
) -> crate::error::Result {
    // sqlx::query!(
    //     "INSERT INTO images (document, url, pagenum) VALUES ($1, $2, $3)",
    //     doc_id,
    //     url,
    //     page
    // )
    // .execute(pool)
    // .await?;

    Ok(())
}

async fn upload_mirror(
    title: &str,
    event: &str,
    year: i32,
    content: &Vec<u8>,
) -> crate::error::Result<String> {
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
    Ok(url)
}

async fn download_file(
    url: &str,
    name: &str,
) -> crate::error::Result<(PathBuf, Vec<u8>)> {
    let request = reqwest::get(url).await?;
    let mut file = File::create(format!("./tmp/{name}.pdf"))?;
    let body = request.bytes().await?;
    file.set_len(body.len() as u64)?;
    file.write_all(&body)?;
    let path = PathBuf::from_str(&format!("./tmp/{name}.pdf"))?;
    // ensure we're actually pointing to a legit file.
    path.try_exists()?;
    Ok((path, body.to_vec()))
}

async fn insert_event(
    pool: &Connection,
    year: i32,
    event: &ParserEvent,
    series: Series,
) -> crate::error::Result<Event> {
    // struct Id {
    //     id: i64,
    // }

    // let mut db_event = Event {
    //     id: None,
    //     series,
    //     year: year as i32,
    //     name: event.title.as_ref().unwrap().clone(),
    //     created: Utc::now(),
    // };
    // let series: String = db_event.series.into();
    // let res: Id = sqlx::query_as_unchecked!(Id, "INSERT INTO events (series, year, name, created, current, new) VALUES ($1, $2, $3, $4, 0, 1) RETURNING id",
    // series,
    // db_event.year,
    // db_event.name,
    // db_event.created).fetch_one(pool).await?;
    // db_event.id = Some(res.id);
    unimplemented!()
}

async fn get_season(
    url: &str,
    year: i32,
) -> crate::error::Result<super::parser::Season> {
    let request = reqwest::get(url).await?;
    let bytes = request.bytes().await?;
    let mut tendril = ByteTendril::new();
    bytes.as_bytes().read_to_tendril(&mut tendril)?;
    let mut input = BufferQueue::default();
    input.push_back(tendril.try_reinterpret().unwrap());
    let mut parser_season = RefCell::new(super::parser::Season {
        year,
        events: vec![],
    });
    let sink = HTMLParser::new(&mut parser_season);
    let opts = TokenizerOpts::default();
    let tok = Tokenizer::new(sink, opts);
    let _ = tok.feed(&mut input);
    tok.end();
    Ok(parser_season.into_inner())
}
