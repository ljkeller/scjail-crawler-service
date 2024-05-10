pub mod error;
pub mod inmate;
pub mod serialize;
mod utils;

use log::{debug, error, info, trace, warn};
use std::env;

// Now, users can just use crate::Error
pub use error::Error;
use inmate::Record;

async fn fetch_inmate_sysids(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<String>, crate::Error> {
    let sys_id_selector =
        scraper::Selector::parse(".inmates-table tr td a[href]").map_err(|_| Error::ParseError)?;
    let mut ret_urls = Vec::new();

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|_| Error::NetworkError)?;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    debug!("Response: {:?} {}", res.version(), res.status());
    let body = res.text().await.map_err(|_| Error::NetworkError)?;
    let document = scraper::Html::parse_document(&body);
    for row in document.select(&sys_id_selector) {
        trace!("Row: {:#?}", row.value());
        if let Some(url_sys_id) = row.value().attr("href") {
            ret_urls.push(url_sys_id.into());
            trace!("Pushed URL Sys ID: {:#?}", url_sys_id);
        } else {
            warn!("No URL Sys ID found in row: {:#?}", row.value());
            continue;
        }
    }

    Ok(ret_urls)
}

pub async fn fetch_records(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<Record>, crate::Error> {
    info!("Fetching records for URL: {url}...");
    let mut records = Vec::new();

    let sys_ids = match fetch_inmate_sysids(client, url).await {
        Ok(sys_ids) => {
            info!("Fetched sys IDs: {:#?} for {url}", sys_ids);
            sys_ids
        }
        Err(e) => {
            error!("Error fetching sys IDs: {:#?} for {url}", e);
            return Err(Error::NetworkError);
        }
    };

    let stop_early = env::var("STOP_EARLY").is_ok();
    for sys_id in sys_ids.iter() {
        let record = Record::build(client, sys_id).await;
        match record {
            Ok(record) => {
                info!("Record: {:#?}", record);
                records.push(record);
            }
            Err(e) => {
                error!("Error building record: {:#?} for {sys_id}. Continuing", e);
            }
        }
        if stop_early {
            info!("'STOP_EARLY' detected- stopping early");
            return Ok(records);
        }
        // TODO: use config value to set sleep duration
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
    Ok(records)
}
