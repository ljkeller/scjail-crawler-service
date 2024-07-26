use log::warn;
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

pub async fn get_last_n_sys_ids(
    n: i64,
    pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<impl Iterator<Item = String> + DoubleEndedIterator + ExactSizeIterator, Error> {
    let sys_ids = sqlx::query!(
        r#"SELECT scil_sysid FROM inmate ORDER BY id DESC LIMIT $1"#,
        n
    )
    .fetch_all(pool)
    .await
    .map_err(|e| Error::PostgresError(format!("failed to get last {} sys_ids: {}", n, e)))?;

    Ok(sys_ids.into_iter().map(|row| {
        row.scil_sysid
            .expect("Expect scil_sysid in get_last_n_sys_ids query")
    }))
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
