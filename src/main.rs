use log::{error, info};

use scjail_crawler_service::fetch_records;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    pretty_env_logger::init();

    let url = if let Some(url) = std::env::args().nth(1) {
        url
    } else {
        "https://www.scottcountyiowa.us/sheriff/inmates.php?comdate=today".into()
    };

    let client = reqwest::Client::new();
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
