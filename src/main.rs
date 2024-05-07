use log::{error, info};
use sqlx::{postgres::PgPoolOptions, Executor};

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
    let sys_ids = fetch_records(&client, &url).await;
    match sys_ids {
        Ok(sys_ids) => {
            info!("Sys IDs: {:#?}", sys_ids);
        }
        Err(e) => {
            error!("Error fetching sys IDs: {:#?}", e);
            return Err(e);
        }
    }

    let pool = pool_res
        .await
        .map_err(|_| Error::InternalError(String::from("Failed to connect to database!")))?;

    let inmates_count = pool
        .execute("SELECT * FROM inmate")
        .await
        .map_err(|_| Error::InternalError(String::from("Failed to execute query!")))?;

    info!("Inmates count: {:#?}", inmates_count);

    Ok(())
}
