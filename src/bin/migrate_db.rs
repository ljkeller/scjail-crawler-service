use scjail_crawler_service::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("This file will be used to migrate sqlite to postgres.");
    Ok(())
}
