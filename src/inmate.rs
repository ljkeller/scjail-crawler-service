use log::{debug, error, info, trace, warn};
use sqlx::Row;

use crate::utils::{cents_to_dollars, dollars_to_cents};
use async_openai::{types::CreateEmbeddingRequestArgs, Client};
use scraper::{Html, Selector};

#[derive(Default)]
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
        tokio::time::sleep(std::time::Duration::from_millis(75)).await;
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
                            self.height = (!dd_text.is_empty()).then(|| dd_text.replace("\\", ""));
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

impl std::fmt::Debug for InmateProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InmateProfile")
            .field("first_name", &self.first_name)
            .field("middle_name", &self.middle_name)
            .field("last_name", &self.last_name)
            .field("affix", &self.affix)
            .field("perm_id", &self.perm_id)
            .field("sex", &self.sex)
            .field("dob", &self.dob)
            .field("arrest_agency", &self.arrest_agency)
            .field("booking_date", &self.booking_date_iso8601)
            .field("booking_number", &self.booking_number)
            .field("height", &self.height)
            .field("weight", &self.weight)
            .field("race", &self.race)
            .field("eye_color", &self.eye_color)
            .field("aliases", &self.aliases)
            .field("scil_sys_id", &self.scil_sys_id)
            .field(
                "img_blob",
                if self.img_blob.is_some() {
                    &"<some blob>"
                } else {
                    &"None"
                },
            )
            .finish()
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
pub struct DbInmateProfile {
    id: i64,
    pub profile: InmateProfile,
}

impl DbInmateProfile {
    pub fn new(id: i64, inmate_profile: InmateProfile) -> DbInmateProfile {
        DbInmateProfile {
            id,
            profile: inmate_profile,
        }
    }
}

//WARN: remove the panicking? Only gonna run this script once or twice
impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for DbInmateProfile {
    /// Create an InmateProfile from a SqliteRow, assuming the row has been joined several times to
    /// aggregate all the necessary data.
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(DbInmateProfile::new(
            row.get("id"),
            InmateProfile {
                first_name: row.get("first_name"),
                middle_name: row.get("middle_name"),
                last_name: row.get("last_name"),
                affix: row.get("affix"),
                perm_id: row.get("permanent_id"),
                sex: row.get("sex"),
                dob: row.get("dob"),
                arrest_agency: row.get("arresting_agency"),
                booking_date_iso8601: row.get("booking_date"),
                booking_number: row.get("booking_number"),
                height: row.get("height"),
                weight: row.get("weight"),
                race: row.get("race"),
                eye_color: row.get("eye_color"),
                aliases: row
                    .get::<Option<String>, _>("aliases")
                    .map(|aliases: String| InmateProfile::get_aliases(&aliases))
                    .flatten(),
                img_blob: row.get("img"),
                scil_sys_id: row.get("scil_sysid"),
                embedding: Option::None,
            },
        ))
    }
}

#[derive(Debug, sqlx::FromRow)]
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

    pub fn get_total_bond_description(&self) -> String {
        let unbondable = self
            .bonds
            .iter()
            .any(|b| b.bond_type.to_lowercase() == "unbondable");
        if unbondable {
            return "unbondable".to_string();
        } else {
            let amount_pennies = self.bonds.iter().map(|b| b.bond_amount).sum::<u64>();
            return cents_to_dollars(amount_pennies);
        }
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

impl std::fmt::Display for ChargeGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChargeGrade::Felony => write!(f, "Felony"),
            ChargeGrade::Misdemeanor => write!(f, "Misdemeanor"),
        }
    }
}

#[derive(Debug)]
pub struct Charge {
    pub description: String,
    pub grade: ChargeGrade,
    pub offense_date: String,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Charge {
    fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Charge {
            description: row.get("description"),
            grade: ChargeGrade::from_string(row.get("grade")),
            offense_date: row.get("offense_date"),
        })
    }
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

    // TODO!
    // pub async fn get_openai_embedding(&self, openai_client: async_openai::Client) {
    //     let request = CreateEmbeddingRequestArgs::default
    //         .model("text-embedding-3-small")
    //         .input(self.generate_embedding_story())
    //         .build()
    //         .expect("Expect embedding request to build");
    // }

    pub fn generate_embedding_story(&self) -> Result<String, crate::Error> {
        let sex_description = match &self.profile.sex {
            Some(sex) => {
                if sex.to_lowercase() == "male" {
                    "man"
                } else {
                    "woman"
                }
            }
            None => "person",
        };

        let alias_description = match &self.profile.aliases {
            Some(aliases) => {
                format!(
                    "{} is known to the following aliases: {}.",
                    self.profile.get_full_name(),
                    aliases.join(", ")
                )
            }
            None => String::from("No known aliases."),
        };

        // TODO: format the date for embeddings
        let intro = format!(
            "A {} {} named {} was arrested on {} by {}.",
            self.profile.race.as_ref().unwrap_or(&"".to_string()),
            sex_description,
            self.profile.get_full_name(),
            self.profile.booking_date_iso8601,
            self.profile
                .arrest_agency
                .as_ref()
                .unwrap_or(&"an unknown agency".to_string())
        );

        let charge_description = format!(
            "Charges include {}. Bond is set at {}.",
            self.charges
                .charges
                .iter()
                .map(|c| c.description.to_string())
                .collect::<Vec<String>>()
                .join(", "),
            self.bond.get_total_bond_description()
        );

        let physical_description = format!(
            "{} is described as {} tall, weighing {}, and having {}. {}",
            self.profile.first_name,
            self.profile
                .height
                .as_ref()
                .unwrap_or(&"unknown height".to_string()),
            self.profile
                .weight
                .as_ref()
                .unwrap_or(&"unkown weight".to_string()),
            self.profile
                .eye_color
                .as_ref()
                .unwrap_or(&"unknown eye color".to_string()),
            alias_description
        );

        let id_description = format!(
            "The inmate's booking number is {}, and their permanent ID is {}.",
            self.profile
                .booking_number
                .as_ref()
                .unwrap_or(&"unknown".to_string()),
            self.profile.perm_id.as_ref().unwrap_or(&"".to_string())
        );

        let story = format!(
            "{} {} {} {}",
            intro, charge_description, physical_description, id_description
        );
        debug!("Generated story: {}", story);

        Ok(story)
    }
}
