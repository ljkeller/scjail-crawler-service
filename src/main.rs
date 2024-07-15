use async_openai::config::OpenAIConfig;
use async_openai::Client as OaiClient;
use log::{info, trace, warn};
use sqlx::postgres::PgPoolOptions;
use std::env;

use scjail_crawler_service::serialize::{create_dbs, serialize_records};
use scjail_crawler_service::{fetch_records, s3_utils, Error};

#[tokio::main]
async fn main() -> Result<(), crate::Error> {
    pretty_env_logger::init();
    info!("Running scjail-crawler-service...");
    info!("Reading (optional) positional arguments: url");
    info!("Reading ENV Vars--\n -required: DATABASE_URL, \n -optional: AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY, OPENAI_API_KEY");

    let pg_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    let pool_res = PgPoolOptions::new().max_connections(5).connect(&pg_url);

    let aws_s3_client = if let Ok(_) = env::var("AWS_ACCESS_KEY_ID") {
        trace!("AWS_ACCESS_KEY_ID found, initializing default S3 client...");
        let (_region, client) = s3_utils::get_default_s3_client().await;
        Some(client)
    } else {
        warn!("No AWS_ACCESS_KEY_ID env var found skipping S3 client initialization... (Only environment variables are supported for this implementation)");
        if let Ok(_) = env::var("AWS_SECRET_ACCESS_KEY") {
            warn!("AWS_SECRET_ACCESS_KEY found, but no AWS_ACCESS_KEY_ID found, skipping S3 client initialization...");
        } else {
            warn!("No AWS_SECRET_ACCESS_KEY found, skipping S3 client initialization...");
        }
        None
    };

    let oai_client = if let Ok(_) = env::var("OPENAI_API_KEY") {
        trace!("OpenAI API key found, initializing client...");
        Some(OaiClient::new())
    } else {
        warn!("No OPENAI_API_KEY env var found, skipping embedding logic...");
        None
    };

    let url = if let Some(url) = std::env::args().nth(1) {
        url
    } else {
        "https://www.scottcountyiowa.us/sheriff/inmates.php?comdate=today".into()
    };

    let client_builder = reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(15));
    let client = client_builder
        .build()
        .map_err(|_| Error::InternalError(String::from("Building reqwest client failed!")))?;

    info!(
        "Established clients: aws: {:?}, openai: {:?}",
        aws_s3_client, oai_client
    );

    let records = fetch_records(&client, &url).await?;

    let pool = pool_res
        .await
        .map_err(|_| Error::InternalError(format!("Failed to connect to database: {}", pg_url)))?;
    create_dbs(&pool).await?;

    serialize_records::<_, OpenAIConfig>(records, &pool, &oai_client, &aws_s3_client).await?;

    Ok(())
}
