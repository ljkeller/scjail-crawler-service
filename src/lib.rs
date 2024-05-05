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

/// Returns the cent value of a given dollar string, assuming the string is in the format of "$x.yz", where x is a non-negative integer and yz are two base 10 digits.
///
/// ## Warning
///
/// This function will break when given negative values, or values without their cents.
///
/// [`unwrap_or_else`]: Result::unwrap_or_else
///
/// # Examples
///
/// ```
/// let dollars = "$2,200.75";
/// assert_eq!(scjail_crawler_service::dollars_to_cents(&dollars), 220075);
///
/// let dollars = "$0.00";
/// assert_eq!(scjail_crawler_service::dollars_to_cents(&dollars), 0);
/// ```
pub fn dollars_to_cents(dollars: &str) -> u64 {
    if let Ok(cents) = dollars
        .chars()
        .filter(|c| c.is_digit(10))
        .collect::<String>()
        .parse::<u64>()
    {
        cents
    } else {
        warn!("Something went wrong parsing {dollars} for cents value. Returning 0.");
        0
    }
}

pub mod inmate {
    use log::{error, info, trace, warn};

    #[derive(Debug)]
    pub struct InmateProfile {}

    impl InmateProfile {
        pub fn build(html: &scraper::Html) -> Result<InmateProfile, crate::Error> {
            trace!("Building InmateProfile from HTML: {:#?}", html);

            Ok(InmateProfile {})
        }
    }

    #[derive(Debug)]
    pub struct Bond {
        pub bond_type: String,
        pub bond_amount: u64,
    }

    #[derive(Debug)]
    pub struct BondInformation {
        pub bonds: Vec<Bond>,
    }

    impl BondInformation {
        pub fn build(html: &scraper::Html) -> Result<BondInformation, crate::Error> {
            let mut bonds = Vec::new();
            // | Date Set | Type ID	| Bond Amt | Status	| Posted By	| Date Posted |
            trace!("Building BondInformation from HTML: {:#?}", html.html());
            let bond_tr_selector = scraper::Selector::parse(".inmates-bond-table tbody tr")
                .map_err(|_| crate::Error::ParseError)?;
            let td_selector =
                scraper::Selector::parse("td").map_err(|_| crate::Error::ParseError)?;

            for row in html.select(&bond_tr_selector) {
                let mut td = row.select(&td_selector);

                let bond_type = match td.nth(1) {
                    Some(td) => td.text().collect::<String>(),
                    None => {
                        warn!("No bond type found in row: {:#?}. Continuing in hope there is a non-corrupt bond type", row);
                        continue;
                    }
                };
                let bond_amount = match td.nth(0) {
                    Some(td) => crate::dollars_to_cents(&td.text().collect::<String>()),
                    None => {
                        warn!("No bond amount found in row: {:#?}. Continuing in hope there is a non-corrupt bond amount", row);
                        continue;
                    }
                };

                bonds.push(Bond {
                    bond_type,
                    bond_amount,
                });
            }

            if bonds.is_empty() {
                error!("No bonds found in HTML: {:#?}", html.html());
            }

            Ok(BondInformation { bonds })
        }
    }

    #[derive(Debug)]
    pub enum ChargeGrade {
        Felony,
        Misdemeanor,
    }

    impl ChargeGrade {
        pub fn from_string(s: &str) -> ChargeGrade {
            match s.to_lowercase().as_str() {
                "felony" => ChargeGrade::Felony,
                "misdemeanor" => ChargeGrade::Misdemeanor,
                _ => {
                    warn!("Unknown charge grade: {:#?}. Defaulting to Misdemeanor", s);
                    ChargeGrade::Misdemeanor
                }
            }
        }
    }

    #[derive(Debug)]
    pub struct Charge {
        pub description: String,
        pub grade: ChargeGrade,
        pub offense_date: String,
    }

    #[derive(Debug)]
    pub struct ChargeInformation {
        pub charges: Vec<Charge>,
    }

    impl ChargeInformation {
        pub fn build(html: &scraper::Html) -> Result<ChargeInformation, crate::Error> {
            trace!("Building ChargeInformation from HTML: {:#?}", html);
            let mut charges = Vec::new();

            let row_selector = scraper::Selector::parse(".inmates-charges-table tbody tr")
                .map_err(|_| crate::Error::ParseError)?;
            let td_selector =
                scraper::Selector::parse("td").map_err(|_| crate::Error::ParseError)?;

            for charge_row in html.select(&row_selector) {
                let mut td = charge_row.select(&td_selector);

                let description = match td.nth(1) {
                    Some(td) => td.text().collect::<String>().trim().to_string(),
                    None => {
                        warn!(
                            "No description found in row: {:#?}. Accepting blank description!",
                            charge_row
                        );
                        String::from("")
                    }
                };

                let grade = match td.nth(0) {
                    Some(grade) => {
                        ChargeGrade::from_string(&grade.text().collect::<String>().trim())
                    }
                    None => {
                        warn!(
                            "No grade found in row: {:#?}. Defaulting to Misdemeanor!",
                            charge_row
                        );
                        ChargeGrade::Misdemeanor
                    }
                };

                let offense_date = match td.nth(0) {
                    Some(date) => date.text().collect::<String>().trim().to_string(),
                    None => {
                        warn!(
                            "No offense date found in row: {:#?}. Assuming date is today!",
                            charge_row
                        );
                        // TODO! Verify this works nicely with postgres
                        chrono::Utc::now().to_string()
                    }
                };

                charges.push(Charge {
                    description,
                    grade,
                    offense_date,
                })
            }

            if charges.is_empty() {
                error!("No charges found in HTML: {:#?}", html.html());
                return Err(crate::Error::ParseError);
            }

            Ok(ChargeInformation { charges })
        }
    }

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
            info!("Building record for URL: {:#?}", request_url);
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
                profile: InmateProfile::build(&record_body_html)?,
                bond: BondInformation::build(&record_body_html)?,
                charges: ChargeInformation::build(&record_body_html)?,
            })
        }
    }
}

use inmate::*;

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
