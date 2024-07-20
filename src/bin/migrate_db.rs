use async_openai::{config::OpenAIConfig, Client as OaiClient};
use log::{error, info, trace, warn};
use sqlx::{postgres::PgPoolOptions, Column, Connection, Row, SqliteConnection, TypeInfo};

use std::env;

use scjail_crawler_service::{
    inmate::{Bond, BondInformation, Charge, ChargeInformation, DbInmateProfile, Record},
    s3_utils,
    serialize::{create_dbs, serialize_records},
    Error,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    pretty_env_logger::init();
    info!("Migrating SQLite database to Postgres...");
    info!("Reading ENV Vars--\n -required: SQLITE_DATABASE, POSTGRES_DATABASE, \n -optional: QUERY_LIMIT");

    let mut sqlite_conn = SqliteConnection::connect(
        &env::var("SQLITE_DATABASE").expect("env variable SQLITE_DATABASE must be set"),
    )
    .await?;

    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(
            &env::var("POSTGRES_DATABASE").expect("env variable POSTGRES_DATABASE must be set"),
        )
        .await?;
    let create_req = create_dbs(&pg_pool);

    let limit: Option<i64> = match env::var("QUERY_LIMIT") {
        Ok(limit) => Some(
            limit
                .parse::<i64>()
                .expect("QUERY_LIMIT must be a valid i64"),
        ),
        Err(_) => None,
    };
    info!("Query limit: {:?}", limit);
    let records: Vec<Record> = get_records_from_sqlite_in_descending_ids(&mut sqlite_conn, &limit)
        .await?
        .rev()
        .collect();

    let aws_s3_client = if let Ok(_) = env::var("AWS_ACCESS_KEY_ID") {
        trace!("AWS_ACCESS_KEY_ID found, initializing default S3 client...");
        let (_region, client) = s3_utils::get_default_s3_client().await;
        Some(client)
    } else {
        warn!("No AWS_ACCESS_KEY_ID env var found skipping S3 client initialization... (Only environment variables are supported for this implementation)");
        if let Ok(_) = env::var("AWS_SECRET_ACCESS_KEY") {
            warn!("AWS_SECRET_ACCESS_KEY found, but no AWS_ACCESS_KEY_ID found, skipping S3 client initialization...");
        } else {
            warn!("No AWS_SECRET_ACCESS_KEY found, skipping S3 client initialization...");
        }
        None
    };

    let oai_client = if let Ok(_) = env::var("OPENAI_API_KEY") {
        trace!("OpenAI API key found, initializing client...");
        Some(OaiClient::new())
    } else {
        warn!("No OpenAI API key found, skipping embedding logic...");
        None
    };

    trace!(
        "Established clients: aws? {:?}, openai? {:?}",
        aws_s3_client.is_some(),
        oai_client.is_some()
    );

    create_req.await?;
    match serialize_records::<_, OpenAIConfig>(records, &pg_pool, &oai_client, &aws_s3_client).await
    {
        Err(e) => error!("Failed to serialize records: {:?}", e),
        _ => info!("Successfully serialized records!"),
    }

    Ok(())
}

async fn get_records_from_sqlite_in_descending_ids(
    conn: &mut SqliteConnection,
    limit: &Option<i64>,
) -> Result<impl Iterator<Item = Record> + DoubleEndedIterator + ExactSizeIterator, Error> {
    let profiles = get_inmate_profiles_sqlite(conn, limit).await?;
    let mut records: Vec<Record> = Vec::new();

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
        trace!("<{:?}>\n", record);
    });
    info!("Number of records: {}", records.len());

    Ok(records.into_iter())
}

/// Query to build a collection of InmateProfile structs.
async fn get_inmate_profiles_sqlite(
    conn: &mut SqliteConnection,
    limit: &Option<i64>,
) -> Result<Vec<DbInmateProfile>, Error> {
    let query = r#"
            SELECT inmate.*, group_concat(alias) as aliases, img.img 
            FROM inmate
            LEFT JOIN inmate_alias ON inmate.id = inmate_alias.inmate_id
            LEFT JOIN img ON inmate.id = img.inmate_id
            LEFT JOIN alias ON inmate_alias.alias_id = alias.id
            GROUP BY inmate.id 
            ORDER BY inmate.id DESC
        "#;
    let query = match limit {
        Some(limit) => format!("{} LIMIT {}", query, limit),
        None => query.to_string(),
    };
    trace!("Get inmate profile Query: {}", query);

    let profiles: Vec<DbInmateProfile> = sqlx::query_as(&query).fetch_all(conn).await?;

    Ok(profiles)
}

async fn get_inmate_bond_information_sqlite(
    conn: &mut SqliteConnection,
    inmate_id: i64,
) -> Result<BondInformation, Error> {
    let query = r#"
            SELECT type, amount_pennies
            FROM bond
            WHERE inmate_id = $1 
        "#;
    trace!("Get inmate bond information Query: {}", query);
    let bonds: Vec<Bond> = sqlx::query_as(query)
        .bind(inmate_id)
        .fetch_all(conn)
        .await?;

    Ok(BondInformation { bonds })
}

async fn get_inmate_charge_information_sqlite(
    conn: &mut SqliteConnection,
    inmate_id: i64,
) -> Result<ChargeInformation, Error> {
    let query = r#"
            SELECT description, grade, offense_date
            FROM charge
            WHERE inmate_id = $1 
        "#;
    trace!("Get inmate charge information Query: {}", query);
    let charges: Vec<Charge> = sqlx::query_as(query)
        .bind(inmate_id)
        .fetch_all(conn)
        .await?;

    Ok(ChargeInformation { charges })
}

/// Perform a query and print the resulting sql rows.
#[allow(dead_code)]
async fn dirty_print_query(query: &str, conn: &mut SqliteConnection) -> Result<(), Error> {
    info!("Query: {}", query);
    let rows = sqlx::query(query).fetch_all(conn).await?;
    for row in rows {
        dirty_print_row(&row).await;
    }

    Ok(())
}

/// Print a SqliteRow, assuming its cols can be decoded as a string.
#[allow(dead_code)]
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
