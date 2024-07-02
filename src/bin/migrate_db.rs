use sqlx::Connection;
use sqlx::Row;
use sqlx::SqliteConnection;
use std::env;

use scjail_crawler_service::inmate::InmateProfile;
use scjail_crawler_service::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("This file will be used to migrate sqlite to postgres.");

    let mut conn = SqliteConnection::connect(
        &env::var("SQLITE_DATABASE").expect("env variable SQLITE_DATABASE must be set"),
    )
    .await?;

    list_inmates(&mut conn).await?;

    println!("\n\n\n");
    list_inmates_full(&mut conn).await?;
    println!("\n\n\n");

    dirty_print_query(r#"
        select inmate.*, group_concat(alias) from inmate left join inmate_alias on inmate.id = inmate_alias.inmate_id left join alias on inmate_alias.alias_id = alias.id group by inmate.id LIMIT 20
        "#,
        &mut conn).await?;
    Ok(())

    // TODO: Goal is to build a record then call serialize_record
    // Next, we'll swap the db out from dev to prod
}

async fn list_inmates(conn: &mut SqliteConnection) -> Result<(), Error> {
    //WARN: some inmate info is missing, as its not all available from one table
    let inmates: Vec<InmateProfile> = sqlx::query_as(
        r#"
        SELECT *
        FROM inmate
        LIMIT 20
    "#,
    )
    .fetch_all(conn)
    .await?;

    for inmate in &inmates {
        println!("{:?}", inmate);
    }

    Ok(())
}

async fn list_inmates_full(conn: &mut SqliteConnection) -> Result<(), Error> {
    //WARN: some inmate info is missing, as its not all available from one table
    let inmates: Vec<InmateProfile> = sqlx::query_as(
        r#"
        select inmate.*, group_concat(alias) from inmate left join inmate_alias on inmate.id = inmate_alias.inmate_id left join alias on inmate_alias.alias_id = alias.id group by inmate.id LIMIT 20
    "#)
    .fetch_all(conn)
    .await?;

    for inmate in &inmates {
        println!("{:?}", inmate);
    }

    Ok(())
}

async fn dirty_print_query(query: &str, conn: &mut SqliteConnection) -> Result<(), Error> {
    let rows = sqlx::query(query).fetch_all(conn).await?;
    for row in rows {
        dirty_print_row(&row).await;
    }

    Ok(())
}

/// Print a SqliteRow, assuming its cols can be decoded as a string.
async fn dirty_print_row(row: &sqlx::sqlite::SqliteRow) {
    print!("<");
    for col_idx in 0..row.len() {
        print!("Col name: {:?}", row.column(col_idx));
        print!(", ");
        print!("Col val: {:?}", row.get_unchecked::<&str, usize>(col_idx));

        if col_idx < row.len() - 1 {
            print!("|");
        }
    }
    println!(">");
}
