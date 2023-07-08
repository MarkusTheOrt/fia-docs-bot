use std::time::Duration;

use serenity::prelude::Context;
use sqlx::{MySql, Pool};

use crate::model::series::RacingSeries;

#[derive(Debug)]
pub struct JoinRes {
    id: u64,
    channel: u64,
    role: Option<u64>,
    threads: bool,
    //   thread: Option<u64>
}

#[derive(Debug)]
pub struct JoinImage {
    id: u64,
    event: u64,
    title: String,
    url: String,
    mirror: String,
    image: String,
    page: u32,
}

#[derive(Debug)]
struct MinImg {
    url: String,
    page: u32,
}

#[derive(Debug)]
pub struct ImageDoc {
    id: u64,
    event: u64,
    title: String,
    url: String,
    mirror: String,
    images: Vec<MinImg>,
}

#[tokio::main]
pub async fn runner(ctx: Context, pool: Pool<MySql>) {
    loop {
        std::thread::sleep(Duration::from_secs(5));
        let data = match join_query(&pool, RacingSeries::F1).await {
            Ok(data) => data,
            Err(why) => {
                eprintln!("Error reading guilds from database:\n{why}");
                continue;
            }
        };
        let docs = match unposted_documents(&pool, RacingSeries::F1).await {
            Ok(data) => data,
            Err(why) => {
                eprintln!("Error reading unposted docs from db:\n{why}");
                continue;
            }
        };
        let docs = join_to_doc(docs);
        println!("docs: {:#?}", docs);
    }
}

fn join_to_doc(join_data: Vec<JoinImage>) -> Vec<ImageDoc> {
    let mut docs: Vec<ImageDoc> = Vec::with_capacity(join_data.len());

    for doc_with_img in join_data.into_iter() {
        if let Some(last_doc) = docs.last_mut() {
            if last_doc.id == doc_with_img.id {
                last_doc.images.push(MinImg {
                    url: doc_with_img.image,
                    page: doc_with_img.page,
                });
                continue;
            }
        }

        docs.push(ImageDoc {
            id: doc_with_img.id,
            event: doc_with_img.event,
            title: doc_with_img.title,
            url: doc_with_img.url,
            mirror: doc_with_img.mirror,
            images: vec![MinImg {
                url: doc_with_img.image,
                page: doc_with_img.page,
            }],
        });
    }
    return docs;
}

async fn unposted_documents(
    pool: &Pool<MySql>,
    racing_series: RacingSeries,
) -> Result<Vec<JoinImage>, sqlx::Error> {
    return sqlx::query_as_unchecked!(
        JoinImage,
        r#"
    SELECT
    documents.`id` as `id!`,
    documents.event as event,
    documents.title,
    documents.url,
    documents.mirror,
    images.url as image,
    images.pagenum as page
    FROM documents
    JOIN images ON document = documents.id
    WHERE series = ? 
    AND notified = 0
    AND done = 1"#,
        Into::<String>::into(racing_series)
    )
    .fetch_all(pool)
    .await;
}

async fn join_query(
    pool: &Pool<MySql>,
    racing_series: RacingSeries,
) -> Result<Vec<JoinRes>, sqlx::Error> {
    match racing_series {
        RacingSeries::F1 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f1_channel as `channel!`,
    f1_threads as threads,
    f1_role as role
    FROM guilds
    WHERE
    f1_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
        RacingSeries::F2 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f2_channel as `channel!`,
    f2_threads as threads,
    f2_role as role
    FROM guilds
    WHERE
    f2_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
        RacingSeries::F3 => {
            return sqlx::query_as_unchecked!(
                JoinRes,
                r#"
    SELECT
    id,
    f3_channel as `channel!`,
    f3_threads as threads,
    f3_role as role
    FROM guilds
    WHERE
    f3_channel IS NOT NULL"#
            )
            .fetch_all(pool)
            .await;
        }
    }
}
