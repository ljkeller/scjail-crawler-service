use async_openai::config::OpenAIConfig;
use async_openai::Client as OaiClient;
use log::{info, trace, warn};
use sqlx::postgres::PgPoolOptions;
use std::env;

use scjail_crawler_service::serialize::{create_dbs, serialize_records};
use scjail_crawler_service::{
    fetch_last_two_days_filtered, fetch_records_filtered, s3_utils, utils::get_blacklist_and_updatelist, serialize::update_null_img_records,
    Error,
};

#[tokio::main]
async fn main() -> Result<(), crate::Error> {
    pretty_env_logger::init();
    info!("Running scjail-crawler-service...");
    info!("Reading (optional) positional arguments: url");
    info!("Reading ENV Vars--\n -required: DATABASE_URL, \n -optional: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, OPENAI_API_KEY, DEV_ENV, REQ_DELAY_MS");

    let pg_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    info!("DATABASE_URL: {}", pg_url);
    let pool_res = PgPoolOptions::new().max_connections(5).connect(&pg_url);

    let aws_s3_client = if let Ok(_) = env::var("AWS_ACCESS_KEY_ID") {
        trace!("AWS_ACCESS_KEY_ID found, initializing default S3 client...");
        let (_region, client) = s3_utils::get_default_s3_client().await;
        Some(client)
    } else {
        warn!("No AWS_ACCESS_KEY_ID env var found for S3 client initialization... (Only environment variables are supported for this implementation)");
        if let Ok(_) = env::var("AWS_SECRET_ACCESS_KEY") {
            warn!("AWS_SECRET_ACCESS_KEY found, but no AWS_ACCESS_KEY_ID found for S3 client initialization... Invalid configuration!");
            panic!("Production requires AWS env vars for S3 client initialization! Check the initial logs for more information.");
        }
        match env::var("DEV_ENV") {
            Ok(_) => {
                warn!("DEV_ENV found, continuing in dev mode...");
                None
            }
            _ => {
                panic!("Production requires AWS env vars for S3 client initialization! Did you mean to run in dev mode? If so, set DEV_ENV.");
            }
        }
    };

    let oai_client = if let Ok(_) = env::var("OPENAI_API_KEY") {
        trace!("OpenAI API key found, initializing client...");
        Some(OaiClient::new())
    } else {
        match env::var("DEV_ENV") {
            Ok(_) => {
                warn!("No OPENAI_API_KEY env var found, but DEV_ENV found. Continuing in dev mode...");
                None
            }
            _ => {
                panic!("No OPENAI_API_KEY env var found- production requires this key! Did you mean to run in dev mode? If so, set DEV_ENV.");
            }
        }
    };

    // Optional application arg: URL to crawl
    let url = std::env::args().nth(1);

    let reqwest_client_builder =
        reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(15));
    let reqwest_client = reqwest_client_builder
        .build()
        .map_err(|_| Error::InternalError(String::from("Building reqwest client failed!")))?;

    info!(
        "Established clients: aws: {:?}, openai: {:?}",
        aws_s3_client, oai_client
    );

    let pool = pool_res.await.map_err(|e| {
        Error::InternalError(format!(
            "Failed to connect to database: {}. e: {}",
            pg_url, e
        ))
    })?;
    create_dbs(&pool).await?;

    let (blacklist, updatelist) = get_blacklist_and_updatelist(45, &pool).await?;
    info!("Found these records to blacklist: {:#?}", blacklist.len());
    info!("Found these records due for update: {:#?}", updatelist);

    let (new_records, update_records) = if let Some(url) = url {
        info!("Fetching records for env URL: {:?}...", url);
        fetch_records_filtered(&reqwest_client, &url, &blacklist, &updatelist).await?
    } else {
        info!("Fetching records for last two days...");
        fetch_last_two_days_filtered(&reqwest_client, &blacklist, &updatelist).await?
    };

    info!("Serializing records...");
    match serialize_records::<_, OpenAIConfig>(new_records, &pool, &oai_client, &aws_s3_client).await {
        Ok(_) => (),
        Err(e) => warn!("Failed serialize_records call. Check logs to view successful inserts or failures: {:?}", e),
    }
    match update_null_img_records(update_records, &pool, &aws_s3_client).await {
        Ok(_) => (),
        Err(e) => warn!("Failed to update null image records: {:?}", e),
    }

    Ok(())
}
