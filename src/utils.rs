use log::{debug, warn};
use std::collections::{HashMap, HashSet};
use std::ops::{Div, Rem};

use crate::Error;

/// Returns the cent value of a given dollar string, assuming the string is in the format of "$x.yz", where x is a non-negative integer and yz are two base 10 digits.
///
/// ## Warning
///
/// This function will break when given negative values, or values without their cents.
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

pub fn cents_to_dollars<T>(cents: T) -> String
where
    T: Div<Output = T> + Rem<Output = T> + From<u8> + Copy + std::fmt::Display,
{
    let dollars = cents / T::from(100);
    format!("${}.{:02}", dollars, cents % T::from(100))
}

/// Returns a tuple containing (HashSet of inmate sys_ids that should be ignored, HashMap of inmate
/// sys_ids that need their pictures updated)
///
/// # Justification
/// The blacklist reduces unnecessary web requests by ignoring already processed records.
/// The updatelist is necessary because sometimes our scraper will find records before their images 
/// are uploaded. This function will help fix those broken records.
pub async fn get_blacklist_and_updatelist(
    n: i64,
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<(HashSet<String>, HashMap<String, i32>), Error> {
    let mut blacklist = HashSet::new();
    let mut updatelist = HashMap::new();

    let recent_records = sqlx::query!(
        r#"
           SELECT id, scil_sysid, img_url
           FROM inmate
           ORDER BY id DESC
           LIMIT $1
        "#,
        n
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::PostgresError(format!("failed to get last {} sys_ids: {}", n, e)))?;

    debug!("Found {:#?} records to check for image updates", recent_records);
    for record in recent_records {
        match record.scil_sysid {
            Some(sys_id) => {
                if record.img_url.is_none() || record.img_url.unwrap().is_empty() {
                    updatelist.insert(sys_id, record.id);
                } else {
                    blacklist.insert(sys_id);
                }
            },
            None => {
                warn!("Found a record with no sys_id: {:#?}", record);
            }
        }
    }

    return Ok((blacklist, updatelist));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dollars_to_cents_positive() {
        let dollars = "$2,200.75";
        assert_eq!(dollars_to_cents(&dollars), 220075);
    }

    #[test]
    fn test_dollars_to_cents_zero() {
        let dollars = "$0.00";
        assert_eq!(dollars_to_cents(&dollars), 0);
    }

    #[test]
    fn test_cents_to_dollars_positive() {
        let cents = 220075;
        assert_eq!(cents_to_dollars(cents), "$2200.75");
    }

    #[test]
    fn test_cents_to_dollars_zero() {
        let cents = 0;
        assert_eq!(cents_to_dollars(cents), "$0.00");
    }

    #[test]
    fn test_cents_to_dollars_large_value() {
        let cents = 1234567890;
        assert_eq!(cents_to_dollars(cents), "$12345678.90");
    }
}
