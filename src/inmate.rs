use log::{error, info, trace, warn};

use crate::utils::dollars_to_cents;
use scraper::{Html, Selector};

#[derive(Debug)]
pub struct InmateProfile {
    pub first_name: String,
    pub middle_name: String,
    pub last_name: String,
    pub affix: String,
    pub perm_id: String,
    pub sex: String,
    pub dob: String,
    pub arrest_agency: String,
    pub booking_date_iso8601: String,
    pub booking_number: String,
    pub height: String,
    pub weight: String,
    pub race: String,
    pub eye_color: String,
    pub aliases: Vec<String>,
    pub img_blob: Vec<u8>,
    pub scil_sys_id: String,
    pub embedding: Vec<f32>,
}

impl InmateProfile {
    pub fn build(html: &Html) -> Result<InmateProfile, crate::Error> {
        trace!("Building InmateProfile from HTML: {:#?}", html);

        Ok(InmateProfile {
            first_name: String::new(),
            middle_name: String::new(),
            last_name: String::new(),
            affix: String::new(),
            perm_id: String::new(),
            sex: String::new(),
            dob: String::new(),
            arrest_agency: String::new(),
            booking_date_iso8601: String::new(),
            booking_number: String::new(),
            height: String::new(),
            weight: String::new(),
            race: String::new(),
            eye_color: String::new(),
            aliases: Vec::new(),
            img_blob: Vec::new(),
            scil_sys_id: String::new(),
            embedding: Vec::new(),
        })
    }

    pub fn get_full_name(&self) -> String {
        let mut name = String::from(&self.first_name);
        if !self.middle_name.is_empty() {
            name.push_str(&format!(" {}", self.middle_name));
        }
        name.push_str(&format!(" {}", self.last_name));

        if !self.affix.is_empty() {
            name.push_str(&format!(", {}", self.affix));
        }

        name
    }

    pub fn get_core_attributes(&self) -> String {
        format!(
            "{} {} dob=[{}] booking date=[{}]",
            self.first_name, self.last_name, self.dob, self.booking_date_iso8601
        )
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
    pub fn build(html: &Html) -> Result<BondInformation, crate::Error> {
        let mut bonds = Vec::new();
        // | Date Set | Type ID	| Bond Amt | Status	| Posted By	| Date Posted |
        trace!("Building BondInformation from HTML: {:#?}", html.html());
        let bond_tr_selector = Selector::parse(".inmates-bond-table tbody tr")
            .map_err(|_| crate::Error::ParseError)?;
        let td_selector = Selector::parse("td").map_err(|_| crate::Error::ParseError)?;

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
                Some(td) => dollars_to_cents(&td.text().collect::<String>()),
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
    pub fn build(html: &Html) -> Result<ChargeInformation, crate::Error> {
        trace!("Building ChargeInformation from HTML: {:#?}", html);
        let mut charges = Vec::new();

        let row_selector = Selector::parse(".inmates-charges-table tbody tr")
            .map_err(|_| crate::Error::ParseError)?;
        let td_selector = Selector::parse("td").map_err(|_| crate::Error::ParseError)?;

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
                Some(grade) => ChargeGrade::from_string(&grade.text().collect::<String>().trim()),
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
        let record_body_html = Html::parse_document(&record_body);
        trace!("Record request body: {:#?}", record_body_html);

        Ok(Record {
            url: request_url,
            profile: InmateProfile::build(&record_body_html)?,
            bond: BondInformation::build(&record_body_html)?,
            charges: ChargeInformation::build(&record_body_html)?,
        })
    }
}
