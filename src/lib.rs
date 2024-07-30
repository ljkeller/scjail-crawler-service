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

const SCOTT_COUNTY_INMATE_TRAVERSAL_ROOT: &str =
    "https://www.scottcountyiowa.us/sheriff/inmates.php";

/// Fetches the inmate sys IDs from the given URL.
/// Returns a vector of sys IDs in the form ["oldest_record", "next_oldest_record", ...,
/// "newest_record
async fn fetch_inmate_sysids_old_to_new(
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
    tokio::time::sleep(std::time::Duration::from_millis(
        env::var("REQ_DELAY_MS")
            .unwrap_or("10000".to_string())
            .parse::<u64>()
            .expect("REQ_DELAY_MS must be a valid u64"),
    ))
    .await;

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

//TODO: Update names to specify ordering, add docs
pub async fn fetch_records(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<Record>, crate::Error> {
    info!("Fetching records for URL: {url}...");
    let mut records = Vec::new();

    let sys_ids = match fetch_inmate_sysids_old_to_new(client, url).await {
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
                debug!("Built record: {:#?}", record);
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
        tokio::time::sleep(std::time::Duration::from_millis(
            env::var("REQ_DELAY_MS")
                .unwrap_or("10000".to_string())
                .parse::<u64>()
                .expect("REQ_DELAY_MS must be a valid u64"),
        ))
        .await;
    }
    Ok(records)
}

//TODO: Update names to specify ordering, add docs
pub async fn fetch_records_filtered(
    client: &reqwest::Client,
    url: &str,
    blacklist: &HashSet<String>,
) -> Result<Vec<Record>, crate::Error> {
    info!("Fetching records for URL: {url}...");
    let mut records = Vec::new();

    let sys_ids = match fetch_inmate_sysids_old_to_new(client, url).await {
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
                debug!("Built record: {:#?}", record);
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

        tokio::time::sleep(std::time::Duration::from_millis(
            env::var("REQ_DELAY_MS")
                .unwrap_or("10000".to_string())
                .parse::<u64>()
                .expect("REQ_DELAY_MS must be a valid u64"),
        ))
        .await;
    }
    Ok(records)
}

/// Fetches the last two days' records from the Scott County Inmate listing
/// and returns a vector of records in the order of [oldest ... newest].
///
/// # Errors
///
/// This function will return an error if there are network or parsing errors.
pub async fn fetch_last_two_days_filtered(
    client: &reqwest::Client,
    last_n_sys_ids: &HashSet<String>,
) -> Result<Vec<Record>, crate::Error> {
    let visit_urls: Vec<String> = get_relative_listings_urls_for_last_two_days(client).await?;
    debug!("Last two days urls: {:#?}", visit_urls);
    let mut records: Vec<Record> = Vec::new();

    // Visit [yesterday_url, today_url]
    for relative_url in visit_urls {
        let day_url = format!("{SCOTT_COUNTY_INMATE_TRAVERSAL_ROOT}{relative_url}");
        let records_for_day = fetch_records_filtered(client, &day_url, last_n_sys_ids).await?;
        records.extend(records_for_day);
    }

    Ok(records)
}

/// Gets the last two days' relative URLs from the Scott County Inmate site.
/// Returns a vector of relative URLs in the form [yesterday_url, today_url]
pub async fn get_relative_listings_urls_for_last_two_days(
    client: &reqwest::Client,
) -> Result<Vec<String>, crate::Error> {
    // Refers to 14 <a> elements housing hrefs to the last 7 days (page repeats itself for now)
    let url_selector =
        scraper::Selector::parse("li.dayselection a").map_err(|_| Error::ParseError)?;
    let mut visit_urls: Vec<String> = Vec::new();

    let res = client
        .get(SCOTT_COUNTY_INMATE_TRAVERSAL_ROOT)
        .send()
        .await
        .map_err(|_| Error::NetworkError)?;

    tokio::time::sleep(std::time::Duration::from_millis(
        env::var("REQ_DELAY_MS")
            .unwrap_or("10000".to_string())
            .parse::<u64>()
            .expect("REQ_DELAY_MS must be a valid u64"),
    ))
    .await;

    debug!("Response: {:?} {}", res.version(), res.status());
    let body = res.text().await.map_err(|_| Error::NetworkError)?;
    let document = scraper::Html::parse_document(&body);

    // take(2) for last two days
    for date_entry in document.select(&url_selector).take(2) {
        if let Some(url) = date_entry.value().attr("href") {
            debug!("Found URL: {url}");
            visit_urls.push(url.to_string());
        }
    }
    visit_urls.reverse();

    Ok(visit_urls)
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_get_last_two_days_urls() {
        let client = reqwest::Client::new();
        let urls = super::get_relative_listings_urls_for_last_two_days(&client)
            .await
            .unwrap();
        assert!(urls.len() > 0);
        for url in urls.iter() {
            assert!(url.contains("comdate"));
        }
    }
}
