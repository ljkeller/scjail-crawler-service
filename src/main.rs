async fn fetch_inmate_sysids(url: &str) -> Result<Vec<String>, reqwest::Error> {
    let res = reqwest::get(url).await?;
    eprintln!("Response: {:?} {}", res.version(), res.status());
    let body = res.text().await?;
    let document = scraper::Html::parse_document(&body);

    let sys_id_selector = scraper::Selector::parse(".inmates-table tr td a[href]")
        .expect("Failed to parse inmates-table for sys_id url");
    let mut urls = Vec::new();
    for row in document.select(&sys_id_selector) {
        let url_sys_id = row.value().attr("href");

        match url_sys_id {
            Some(url) => urls.push(url.into()),
            None => {
                println!("No URL Sys ID found in row: {:#?}", row.value());
                continue;
            }
        }
    }

    Ok(urls)
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let url = if let Some(url) = std::env::args().nth(1) {
        url
    } else {
        println!("No CLI URL provided, using default");
        "https://www.scottcountyiowa.us/sheriff/inmates.php?comdate=today".into()
    };

    eprintln!("Fetching URL: {url:?}...");
    let sys_ids = fetch_inmate_sysids(&url).await?;
    println!("Sys IDs: {:#?}", sys_ids);

    Ok(())
}
