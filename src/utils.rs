use log::warn;

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
