use log::{error, info};

use scjail_crawler_service::{fetch_records, Error};

#[tokio::main]
async fn main() -> Result<(), crate::Error> {
    pretty_env_logger::init();

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

    Ok(())
}
