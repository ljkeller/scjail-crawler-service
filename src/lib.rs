pub mod error;
pub mod inmate;
pub mod s3_utils;
pub mod serialize;
pub mod utils;

use log::{debug, error, info, trace, warn};
use std::collections::HashSet;
use std::env;

// Now, users can just use crate::Error
pub use error::Error;
use inmate::Record;

/// Fetches the inmate sys IDs from the given URL.
/// Returns a vector of sys IDs in the form ["oldest_record", "next_oldest_record", ...,
/// "newest_record
async fn fetch_inmate_sysids(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<String>, crate::Error> {
    // Return order is newest records to oldest (for now)
    let sys_id_selector =
        scraper::Selector::parse(".inmates-table tr td a[href]").map_err(|_| Error::ParseError)?;
    let mut ret_urls = Vec::new();

    let res = client
        .get(url)
        .send()
        .await
        .map_err(|_| Error::NetworkError)?;
    tokio::time::sleep(std::time::Duration::from_millis(75)).await;

    debug!("Response: {:?} {}", res.version(), res.status());
    let body = res.text().await.map_err(|_| Error::NetworkError)?;
    let document = scraper::Html::parse_document(&body);
    // Reverse the order of the sys IDs to get the oldest records first, therefore
    // newest records will have biggest db ids
    for row in document.select(&sys_id_selector).rev() {
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
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }
    Ok(records)
}

pub async fn fetch_records_filtered(
    client: &reqwest::Client,
    url: &str,
    blacklist: &HashSet<String>,
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

    // TODO: move the set difference up & split out common code (from fetch_records())
    let stop_early = env::var("STOP_EARLY").is_ok();
    for sys_id in sys_ids.iter() {
        if blacklist.contains(sys_id) {
            info!("Skipping blacklisted sys_id: {sys_id}");
            continue;
        }

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
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
    }
    Ok(records)
}
