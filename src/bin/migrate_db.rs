use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::env;

use scjail_crawler_service::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("This file will be used to migrate sqlite to postgres.");

    let sqlite_pool = SqlitePool::connect(
        &env::var("SQLITE_DATABASE").expect("env variable SQLITE_DATABASE must be set"),
    )
    .await?;

    list_inmates(&sqlite_pool).await?;

    Ok(())
}

async fn list_inmates(pool: &SqlitePool) -> Result<(), Error> {
    let inmates = sqlx::query(
        r#"
        SELECT *
        FROM inmate
        LIMIT 20
    "#,
    )
    .fetch_all(pool)
    .await?;

    for inmate in &inmates {
        dirty_print_row(inmate).await;
    }

    Ok(())
}

async fn dirty_print_row(row: &sqlx::sqlite::SqliteRow) {
    print!("<");
    for col_idx in 0..row.len() {
        print!("{:?}", row.get_unchecked::<&str, usize>(col_idx));

        if col_idx < row.len() - 1 {
            print!("|");
        }
    }
    println!(">");
}
