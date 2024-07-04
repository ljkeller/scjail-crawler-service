use sqlx::{Column, Connection, Row, SqliteConnection, TypeInfo};
use std::env;

use scjail_crawler_service::inmate::{
    Bond, BondInformation, Charge, ChargeInformation, DbInmateProfile, Record,
};
use scjail_crawler_service::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("This file will be used to migrate sqlite to postgres.");

    let mut conn = SqliteConnection::connect(
        &env::var("SQLITE_DATABASE").expect("env variable SQLITE_DATABASE must be set"),
    )
    .await?;

    let records = get_records_from_sqlite(&mut conn).await?;

    Ok(())

    // TODO: Goal is to build a record then call serialize_record
    // Next, we'll swap the db out from dev to prod
}

async fn get_records_from_sqlite(conn: &mut SqliteConnection) -> Result<(), Error> {
    let profiles = get_inmate_profiles_sqlite(conn).await?;
    let mut records = Vec::new();

    for profile in profiles {
        let bond_info = get_inmate_bond_information_sqlite(conn, profile.id).await?;
        let charge_info = get_inmate_charge_information_sqlite(conn, profile.id).await?;
        records.push(Record {
            url: String::from(""),
            profile: profile.profile,
            bond: bond_info,
            charges: charge_info,
        })
    }

    records.iter().for_each(|record| {
        println!("<{:?}>\n", record);
    });

    println!("Number of records: {}", records.len());
    Ok(())
}

/// Query to build a collection of InmateProfile structs.
async fn get_inmate_profiles_sqlite(
    conn: &mut SqliteConnection,
) -> Result<Vec<DbInmateProfile>, Error> {
    let profiles: Vec<DbInmateProfile> = sqlx::query_as(
        r#"
            SELECT inmate.*, group_concat(alias) as aliases, img.img 
            FROM inmate
            LEFT JOIN inmate_alias ON inmate.id = inmate_alias.inmate_id
            LEFT JOIN img ON inmate.id = img.inmate_id
            LEFT JOIN alias ON inmate_alias.alias_id = alias.id
            GROUP BY inmate.id 
        "#,
    )
    .fetch_all(conn)
    .await?;

    Ok(profiles)
}

async fn get_inmate_bond_information_sqlite(
    conn: &mut SqliteConnection,
    inmate_id: i64,
) -> Result<BondInformation, Error> {
    let bonds: Vec<Bond> = sqlx::query_as(
        r#"
            SELECT type, amount_pennies
            FROM bond
            WHERE inmate_id = $1 
        "#,
    )
    .bind(inmate_id)
    .fetch_all(conn)
    .await?;

    Ok(BondInformation { bonds })
}

async fn get_inmate_charge_information_sqlite(
    conn: &mut SqliteConnection,
    inmate_id: i64,
) -> Result<ChargeInformation, Error> {
    let charges: Vec<Charge> = sqlx::query_as(
        r#"
            SELECT description, grade, offense_date
            FROM charge
            WHERE inmate_id = $1 
        "#,
    )
    .bind(inmate_id)
    .fetch_all(conn)
    .await?;

    Ok(ChargeInformation { charges })
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
