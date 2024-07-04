use sqlx::{Column, Connection, Row, SqliteConnection, TypeInfo};
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

    dirty_print_query(r#"
        select inmate.*, group_concat(alias) as aliases, img.img from inmate left join inmate_alias on inmate.id = inmate_alias.inmate_id left join img on inmate.id = img.inmate_id left join alias on inmate_alias.alias_id = alias.id group by inmate.id LIMIT 20;
        "#,
        &mut conn).await?;
    println!("\n\n\nPack structs from query:");

    query_inmate_profiles(&mut conn).await?;

    Ok(())

    // TODO: Goal is to build a record then call serialize_record
    // Next, we'll swap the db out from dev to prod
}

/// Query to build a collection of InmateProfile structs.
async fn query_inmate_profiles(conn: &mut SqliteConnection) -> Result<(), Error> {
    //WARN: some inmate info is missing, as its not all available from one table
    let inmates: Vec<InmateProfile> = sqlx::query_as(
        r#"
        select inmate.*, group_concat(alias) as aliases, img.img from inmate left join inmate_alias on inmate.id = inmate_alias.inmate_id left join img on inmate.id = img.inmate_id left join alias on inmate_alias.alias_id = alias.id group by inmate.id LIMIT 20;
    "#)
    .fetch_all(conn)
    .await?;

    for inmate in &inmates {
        println!("{:?}", inmate);
    }

    Ok(())
}

/// Perform a query and print the resulting sql rows.
async fn dirty_print_query(query: &str, conn: &mut SqliteConnection) -> Result<(), Error> {
    println!("Query: {}", query);
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
        let col_type_info = row.column(col_idx).type_info();
        match col_type_info.name() {
            "BLOB" => print!("Col val: <BLOB>"),
            _ => print!("Col val: {:?}", row.get_unchecked::<&str, usize>(col_idx)),
        }

        if col_idx < row.len() - 1 {
            print!("|");
        }
    }
    println!(">");
}
