use log::{error, info, trace, warn};

use crate::utils::dollars_to_cents;
use scraper::{Html, Selector};

#[derive(Debug, Default)]
pub struct InmateProfile {
    pub first_name: String,
    pub middle_name: Option<String>,
    pub last_name: String,
    pub affix: Option<String>,
    pub perm_id: Option<String>,
    pub sex: Option<String>,
    pub dob: String,
    pub arrest_agency: Option<String>,
    pub booking_date_iso8601: String,
    pub booking_number: Option<String>,
    pub height: Option<String>,
    pub weight: Option<String>,
    pub race: Option<String>,
    pub eye_color: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub img_blob: Option<Vec<u8>>,
    pub scil_sys_id: Option<String>,
    pub embedding: Option<Vec<f32>>,
}

impl InmateProfile {
    pub async fn build(
        html: &Html,
        sys_id: &str,
        client: &reqwest::Client,
    ) -> Result<InmateProfile, crate::Error> {
        trace!("Building InmateProfile from HTML: {:#?}", html);

        // fire off img download request before parsing HTML
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let img_selector = Selector::parse(".inmates img").map_err(|_| crate::Error::ParseError)?;
        let img = if let Some(img_url) = html
            .select(&img_selector)
            .next()
            .and_then(|img| img.attr("src"))
        {
            let full_img_url = format!("https:{}", img_url);
            info!("Found img URL: {:#?}", full_img_url);
            Some(client.get(full_img_url).send())
        } else {
            None
        };

        let mut profile = InmateProfile::default();
        profile.scil_sys_id = Some(sys_id.to_string());
        profile.set_core_profile_data(html)?;

        // Not every inmate will have an image,
        if let Some(img) = img {
            match img.await {
                Ok(img_resp) => {
                    if let Ok(img_blob) = img_resp.bytes().await {
                        profile.img_blob = Some(img_blob.to_vec());
                    }
                }
                Err(e) => warn!("Error fetching img: {:#?}, ignoring...", e),
            }
        }

        // TODO! Get and set embedding
        if profile.first_name.is_empty()
            || profile.last_name.is_empty()
            || profile.dob.is_empty()
            || profile.booking_date_iso8601.is_empty()
        {
            error!("Building a profile requires core attributes: first name, last name, dob, booking date. Current core attributes: {:#?}", profile.get_core_attributes());
            return Err(crate::Error::ParseError);
        }

        Ok(profile)
    }

    fn get_aliases(aliases: &str) -> Option<Vec<String>> {
        let alias_vec = aliases
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<String>>();

        match alias_vec.len() {
            0 => return None,
            _ => return Some(alias_vec),
        }
    }

    fn set_core_profile_data(&mut self, html: &Html) -> Result<(), crate::Error> {
        let num_dts_of_interest = 15;
        let mut found_dts = 0;

        let profile_selector =
            Selector::parse(".table-display").map_err(|_| crate::Error::ParseError)?;
        let dt_selector = Selector::parse("dt").map_err(|_| crate::Error::ParseError)?;
        let dd_selector = Selector::parse("dd").map_err(|_| crate::Error::ParseError)?;
        for table in html.select(&profile_selector) {
            let mut dts = table.select(&dt_selector);
            let mut dds = table.select(&dd_selector);

            // Because dt and dd come in pairs, we can effectively iterate them as a zip(dt, dd)
            // dt is the key, and dd is the value
            while let (Some(dt), Some(dd)) = (dts.next(), dds.next()) {
                if let Some(dt_text) = dt.text().next() {
                    // Sometimes, dd will be empty. For example, when an inmate has no middle name.
                    let dd_text = dd.text().next().unwrap_or_default().trim().to_string();
                    match dt_text.trim().to_ascii_lowercase().as_str() {
                        "first:" => {
                            self.first_name = dd_text;
                            found_dts += 1;
                        }
                        "middle:" => {
                            self.middle_name = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "last:" => {
                            self.last_name = dd_text;
                            found_dts += 1;
                        }
                        "affix:" => {
                            self.affix = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "permanent id:" => {
                            self.perm_id = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "sex:" => {
                            self.sex = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "date of birth:" => {
                            self.dob = dd_text;
                            found_dts += 1;
                        }
                        "height:" => {
                            self.height = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "weight:" => {
                            self.weight = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "race:" => {
                            self.race = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "eye color:" => {
                            self.eye_color = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "alias(es):" => {
                            self.aliases = InmateProfile::get_aliases(&dd_text);
                            found_dts += 1;
                        }
                        "committing agency:" => {
                            self.arrest_agency = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        "booking date time:" => {
                            self.booking_date_iso8601 = dd_text;
                            found_dts += 1;
                        }
                        "booking number:" => {
                            self.booking_number = (!dd_text.is_empty()).then(|| dd_text);
                            found_dts += 1;
                        }
                        _ => {
                            // Do nothing, because we've already advanced dt and dd iterators
                            continue;
                        }
                    }
                } else {
                    warn!("No text found in dt: {:#?}. Skipping...", dt);
                    continue;
                }
            }
        }

        if found_dts != num_dts_of_interest {
            warn!(
                "Found {} data points of interest, expected {}. Continuing...",
                found_dts, num_dts_of_interest
            );
        }
        Ok(())
    }

    pub fn get_full_name(&self) -> String {
        let mut name = String::from(&self.first_name);
        if let Some(middle) = &self.middle_name {
            name.push_str(&format!(" {}", middle));
        }
        name.push_str(&format!(" {}", self.last_name));

        if let Some(affix) = &self.affix {
            name.push_str(&format!(", {}", affix));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_aliases_basic() {
        let aliases = "John Doe, Jane Doe";
        assert_eq!(
            InmateProfile::get_aliases(aliases),
            Some(vec!["John Doe".to_string(), "Jane Doe".to_string()])
        );

        let aliases = "John, Jane Doe, Bob, Bobby, Bobert, Bob er tin a";
        assert_eq!(
            InmateProfile::get_aliases(aliases),
            Some(vec![
                "John".to_string(),
                "Jane Doe".to_string(),
                "Bob".to_string(),
                "Bobby".to_string(),
                "Bobert".to_string(),
                "Bob er tin a".to_string()
            ])
        );
    }

    #[test]
    fn test_get_aliases_advanced() {
        let aliases = "John Doe, , , Jane Doe,    Marty McFly,  ";
        assert_eq!(
            InmateProfile::get_aliases(aliases),
            Some(vec![
                "John Doe".to_string(),
                "Jane Doe".to_string(),
                "Marty McFly".to_string()
            ])
        );
    }

    #[test]
    fn test_aliases_empty_basic() {
        let aliases = "";
        assert_eq!(InmateProfile::get_aliases(aliases), None);
        let aliases = "             ";
        assert_eq!(InmateProfile::get_aliases(aliases), None);
        let aliases = ",";
        assert_eq!(InmateProfile::get_aliases(aliases), None);
    }

    #[test]
    fn test_alias_empty_advanced() {
        let aliases = ",,, ,   ,";
        assert_eq!(InmateProfile::get_aliases(aliases), None);
        let aliases = " , ";
        assert_eq!(InmateProfile::get_aliases(aliases), None);
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
    pub async fn build(client: &reqwest::Client, sys_id: &str) -> Result<Record, crate::Error> {
        let request_url = format!(
            "https://www.scottcountyiowa.us/sheriff/inmates.php{}",
            sys_id
        );
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
            profile: InmateProfile::build(&record_body_html, sys_id, client).await?,
            bond: BondInformation::build(&record_body_html)?,
            charges: ChargeInformation::build(&record_body_html)?,
        })
    }
}
