use log::{debug, info, trace, warn};

pub(crate) async fn fetch_inmate_sysids(url: &str) -> Result<Vec<String>, reqwest::Error> {
    info!("Fetching URL: {url}...");

    let sys_id_selector = scraper::Selector::parse(".inmates-table tr td a[href]")
        .expect("Failed to parse inmates-table for sys_id url");
    let mut ret_urls = Vec::new();

    let res = reqwest::get(url).await?;
    debug!("Response: {:?} {}", res.version(), res.status());
    let body = res.text().await?;
    let document = scraper::Html::parse_document(&body);
    for row in document.select(&sys_id_selector) {
        trace!("Row: {:#?}", row.value());
        let url_sys_id = row.value().attr("href");

        match url_sys_id {
            Some(url) => {
                ret_urls.push(url.into());
                trace!("Pushed URL Sys ID: {:#?}", url);
            }
            None => {
                warn!("No URL Sys ID found in row: {:#?}", row.value());
                continue;
            }
        }
    }

    Ok(ret_urls)
}
