use std::fmt;

use log::{debug, error, info, trace, warn};
use std::env;

#[derive(Debug)]
pub enum Error {
    NetworkError,
    ParseError,
    ArgumentError,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::NetworkError => write!(f, "Network error"),
            Error::ParseError => write!(f, "Parse error"),
            Error::ArgumentError => write!(f, "Argument error"),
        }
    }
}

pub mod inmate {
    use log::trace;

    #[derive(Debug)]
    pub struct InmateProfile {}

    impl InmateProfile {
        pub async fn build(html: &str) -> Result<InmateProfile, crate::Error> {
            Ok(InmateProfile {})
        }
    }

    #[derive(Debug)]
    pub struct BondInformation {}

    #[derive(Debug)]
    pub struct ChargeInformation {}

    #[derive(Debug)]
    pub struct Record {
        pub url: String,
        pub profile: InmateProfile,
        pub bond: BondInformation,
        pub charges: ChargeInformation,
    }

    impl Record {
        // We should probably update this code to return an option type
        // There is so many different ways to fail here, we can write our own error types, or just return an option
        pub async fn build(client: &reqwest::Client, url: &str) -> Result<Record, crate::Error> {
            let request_url = format!("https://www.scottcountyiowa.us/sheriff/inmates.php{}", url);
            let record_body = client
                .get(&request_url)
                .send()
                .await
                .map_err(|_| crate::Error::NetworkError)?
                .text()
                .await
                .map_err(|_| crate::Error::NetworkError)?;
            let record_body_html = scraper::Html::parse_document(&record_body);
            trace!("Record request body: {:#?}", record_body_html);

            Ok(Record {
                url: request_url,
                profile: InmateProfile {},
                bond: BondInformation {},
                charges: ChargeInformation {},
            })
        }
    }
}

use inmate::*;

async fn fetch_inmate_sysids(
    client: &reqwest::Client,
    url: &str,
) -> Result<Vec<String>, crate::Error> {
    let sys_id_selector = scraper::Selector::parse(".inmates-table tr td a[href]")
        .expect("Failed to parse inmates-table for sys_id url");
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
