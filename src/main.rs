use async_openai::{types::CreateEmbeddingRequestArgs, Client as OaiClient};
use log::{info, warn};
use sqlx::postgres::PgPoolOptions;
use std::env;

use scjail_crawler_service::serialize::{create_dbs, inmate_count, serialize_record};
use scjail_crawler_service::{fetch_records, Error};

#[tokio::main]
async fn main() -> Result<(), crate::Error> {
    pretty_env_logger::init();
    let pg_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    let pool_res = PgPoolOptions::new().max_connections(5).connect(&pg_url);

    let url = if let Some(url) = std::env::args().nth(1) {
        url
    } else {
        "https://www.scottcountyiowa.us/sheriff/inmates.php?comdate=today".into()
    };

    let client_builder = reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(15));
    let client = client_builder
        .build()
        .map_err(|_| Error::InternalError(String::from("Building reqwest client failed!")))?;

    let oai_client = OaiClient::new();

    let records = if let Ok(records) = fetch_records(&client, &url).await {
        records
    } else {
        return Err(Error::InternalError(String::from(
            "Failed to fetch records!",
        )));
    };

    let pool = pool_res
        .await
        .map_err(|_| Error::InternalError(format!("Failed to connect to database: {}", pg_url)))?;
    create_dbs(&pool).await?;

    let (mut inserted_count, mut failed_count) = (0, 0);

    for mut record in records {
        info!(
            "Inserting record: {:#?}",
            record.generate_embedding_story()?
        );
        let oai_request = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-3-small") //text-embedding-3-small defaults to 1536 dimensions
            .input(record.generate_embedding_story()?)
            .build()
            .map_err(|_| Error::InternalError(String::from("Failed to build OpenAI request!")))?;

        if env::var("GET_EMBEDDINGS").is_ok() {
            let embed_resp = oai_client.embeddings().create(oai_request).await.unwrap();
            info!("OpenAI embedding resp: {:#?}", embed_resp);

            // Since we are only sending one input per request, we can assume the first embedding
            // is the one we want
            record.profile.embedding = match embed_resp.data.first() {
                Some(embedding) => Some(embedding.embedding.clone()),
                None => {
                    warn!(
                        "No embeddings found in response: {:#?}. Still serializing",
                        embed_resp
                    );
                    None
                }
            };
        }

        match serialize_record(record, &pool).await {
            Ok(_) => {
                inserted_count += 1;
            }
            Err(e) => {
                warn!("Failed to serialize record {:#?}", e);
                failed_count += 1;
            }
        }
    }

    info!(
        "Inserted {} records, failed to insert {} records. Total records: {}",
        inserted_count,
        failed_count,
        inmate_count(&pool).await?
    );

    Ok(())
}
