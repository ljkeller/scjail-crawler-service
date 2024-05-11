use log::{info, warn};
use sqlx::{postgres::PgPoolOptions, query};

use scjail_crawler_service::serialize::{create_dbs, serialize_record};
use scjail_crawler_service::{fetch_records, Error};

#[tokio::main]
async fn main() -> Result<(), crate::Error> {
    pretty_env_logger::init();
    let pool_res = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:123@localhost:5432");

    let url = if let Some(url) = std::env::args().nth(1) {
        url
    } else {
        "https://www.scottcountyiowa.us/sheriff/inmates.php?comdate=today".into()
    };

    let client_builder = reqwest::ClientBuilder::new().timeout(std::time::Duration::from_secs(15));
    let client = client_builder
        .build()
        .map_err(|_| Error::InternalError(String::from("Building reqwest client failed!")))?;

    let records = if let Ok(records) = fetch_records(&client, &url).await {
        records
    } else {
        return Err(Error::InternalError(String::from(
            "Failed to fetch records!",
        )));
    };

    let pool = pool_res
        .await
        .map_err(|_| Error::InternalError(String::from("Failed to connect to database!")))?;
    create_dbs(&pool).await?;

    for record in records {
        match serialize_record(record, &pool).await {
            Ok(id) => {
                info!("Record serialized with id: {:#?}", id);
            }
            Err(e) => {
                warn!("Failed to serialize record {:#?}", e);
            }
        }
    }

    let inmates_count = query!("SELECT COUNT(*) FROM inmate")
        .fetch_one(&pool)
        .await
        .map_err(|_| Error::InternalError(String::from("Failed to execute query!")))?;

    info!("Total inmates count: {:#?}", inmates_count);

    Ok(())
}
