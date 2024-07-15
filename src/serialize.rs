use async_openai::config::{Config, OpenAIConfig};
use async_openai::Client;
use aws_sdk_s3::Client as S3Client;
use log::{debug, info, trace, warn};
use sqlx::postgres::PgPool;
use sqlx::Row;

use crate::inmate::{Bond, Charge, InmateProfile, Record};
use crate::s3_utils;
use crate::Error;

pub async fn create_dbs(pool: &PgPool) -> Result<(), Error> {
    info!("Creating databases if not already existing...");
    create_inmate(pool).await?;
    create_alias(pool).await?;
    create_bond(pool).await?;
    create_charge(pool).await?;
    create_img(pool).await?;
    create_inmate_alias(pool).await?;

    info!("Databases created successfully!");
    Ok(())
}

async fn run_sql_batch(
    pool: &sqlx::Pool<sqlx::Postgres>,
    statements: &Vec<&str>,
) -> Result<(), Error> {
    for statement in statements {
        debug!("Running statement: {}", statement);
        sqlx::query(statement).execute(pool).await.expect(&format!(
            "Expect run sql batch statements. Failed on statement: {}",
            statement
        ));
    }

    Ok(())
}

async fn create_inmate_alias(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Error> {
    sqlx::query!(
        r#"CREATE TABLE IF NOT EXISTS inmate_alias (
          inmate_id INTEGER NOT NULL,
          alias_id INTEGER NOT NULL,
          FOREIGN KEY (inmate_id) REFERENCES inmate(id),
          FOREIGN KEY (alias_id) REFERENCES alias(id),
          PRIMARY KEY (inmate_id, alias_id)
        );"#
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_img(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Error> {
    let statements = vec![
        r#"CREATE TABLE IF NOT EXISTS img (
          id SERIAL PRIMARY KEY,
          inmate_id INTEGER NOT NULL,
          img BYTEA,
          FOREIGN KEY (inmate_id) REFERENCES inmate(id) 
        );"#,
        r#"CREATE INDEX IF NOT EXISTS idx_img_inmate_id ON img(inmate_id);"#,
    ];
    run_sql_batch(pool, &statements).await
}

async fn create_charge(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Error> {
    let statements = vec![
        r#"CREATE TABLE IF NOT EXISTS charge (
          id SERIAL PRIMARY KEY,
          inmate_id INTEGER,
          description TEXT,
          grade TEXT,
          offense_date TEXT,
          FOREIGN KEY (inmate_id) REFERENCES inmate(id)
        );"#,
        r#"CREATE INDEX IF NOT EXISTS idx_inmate_id ON charge(inmate_id);"#,
    ];
    run_sql_batch(pool, &statements).await
}

async fn create_bond(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Error> {
    let statements = vec![
        r#"CREATE TABLE IF NOT EXISTS bond (
          id SERIAL PRIMARY KEY,
          inmate_id INTEGER NOT NULL,
          type TEXT NOT NULL,
          amount_pennies INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (inmate_id) REFERENCES inmate(id) 
        );"#,
        r#"CREATE INDEX IF NOT EXISTS bond_inmate_id_idx ON bond(inmate_id);"#,
    ];

    run_sql_batch(pool, &statements).await
}

async fn create_alias(pool: &sqlx::Pool<sqlx::Postgres>) -> Result<(), Error> {
    sqlx::query!(
        r#"CREATE TABLE IF NOT EXISTS alias (
            id SERIAL PRIMARY KEY,
            alias TEXT UNIQUE NOT NULL CHECK (alias <> '')
        );"#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_inmate(pool: &PgPool) -> Result<(), Error> {
    let statements = vec![
        r#"CREATE EXTENSION IF NOT EXISTS vector;"#,
        r#"CREATE TABLE IF NOT EXISTS inmate (
          id SERIAL PRIMARY KEY,
          first_name TEXT NOT NULL CHECK (first_name <> ''),
          middle_name TEXT,
          last_name TEXT NOT NULL CHECK (last_name <> ''),
          affix TEXT,
          permanent_id TEXT,
          sex TEXT,
          dob date NOT NULL,
          arresting_agency TEXT,
          booking_date TIMESTAMP WITH TIME ZONE NOT NULL,
          booking_number TEXT,
          height TEXT,
          weight TEXT,
          race TEXT,
          eye_color TEXT,
          img_url TEXT,
          scil_sysid TEXT,
          record_visits INTEGER DEFAULT 0,
          shared INTEGER DEFAULT 0,
          embedding vector(1536),
          UNIQUE (first_name, last_name, dob, booking_date)
        );"#,
        r#"CREATE INDEX IF NOT EXISTS idx_inmate_first_name ON inmate(first_name);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_inmate_middle_name ON inmate(middle_name);"#,
        r#"CREATE INDEX IF NOT EXISTS idx_inmate_last_name ON inmate(last_name);"#,
    ];
    run_sql_batch(pool, &statements).await
}

pub async fn inmate_count(pool: &PgPool) -> Result<i64, Error> {
    let res = sqlx::query!("SELECT COUNT(*) FROM inmate")
        .fetch_one(pool)
        .await?;
    Ok(res
        .count
        .expect("Expect count to be present on on inmate count query"))
}

pub async fn serialize_records<I, C>(
    records: I,
    pool: &PgPool,
    oai_client: &Option<Client<OpenAIConfig>>,
    aws_s3_client: &Option<S3Client>,
) -> Result<(), Error>
where
    I: IntoIterator<Item = crate::inmate::Record>,
    C: Config,
{
    info!("Serializing records...");
    let (mut inserted_count, mut failed_count) = (0, 0);
    for (idx, mut record) in records.into_iter().enumerate() {
        trace!(
            "Inserting record: {:#?}",
            record.generate_embedding_story()?
        );

        if record.profile.embedding.is_none() && oai_client.is_some() {
            if let Err(e) = record
                .gather_openai_embedding(oai_client.as_ref().unwrap())
                .await
            {
                warn!(
                    "Failed to gather OpenAI embedding: {:#?}. Continuing serialize.",
                    e
                );
            }
        }

        match serialize_record(record, pool, aws_s3_client).await {
            Ok(_) => {
                inserted_count += 1;
            }
            Err(e) => {
                warn!("Failed to serialize record {:#?}", e);
                failed_count += 1;
            }
        }

        if idx % 25 == 0 {
            info!("Processed {} records", idx);
        }
    }

    info!(
        "Inserted {} records, failed to insert {} records. Total records: {}. OpenAI querying enabled? {}",
        inserted_count,
        failed_count,
        inmate_count(pool).await?,
        oai_client.is_some()
    );
    Ok(())
}

pub async fn serialize_record(
    record: Record,
    pool: &PgPool,
    aws_s3_client: &Option<S3Client>,
) -> Result<i32, Error> {
    trace!("Serializing record: {:#?}", record);
    let mut transaction = pool.begin().await?;
    let inmate_info = record.profile.get_core_attributes();
    let inmate_id = serialize_profile(record.profile, &mut transaction, aws_s3_client).await?;

    for bond in record.bond.bonds {
        serialize_bond(bond, &inmate_id, &mut transaction).await?;
    }

    for charge in record.charges.charges {
        serialize_charge(charge, &inmate_id, &mut transaction).await?;
    }

    // Commit transaction, otherwise implicity rollback on out of scope
    transaction.commit().await?;

    debug!(
        "Successfully serialized {} yielding inmate_id: {}.",
        inmate_info, inmate_id
    );
    Ok(inmate_id)
}

async fn serialize_bond(
    bond: Bond,
    inmate_id: &i32,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), Error> {
    // Could do bulk insert here: https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-bind-an-array-to-a-values-clause-how-can-i-do-bulk-inserts
    // But, there is a low amount of bonds per inmate; therefores, its probably overengineering
    sqlx::query!(
        r#"
        INSERT INTO bond
            (inmate_id, type, amount_pennies)
        VALUES
            ($1, $2, $3)
        "#,
        inmate_id,
        bond.bond_type,
        bond.bond_amount as i32 // TODO: update schema to use i64? bonds are in pennies, so a few billion is possible (I think?) It would be historic...
    )
    .execute(&mut **transaction)
    .await?;

    trace!("Bond serialized: {:#?}", bond);
    Ok(())
}

async fn serialize_charge(
    charge: Charge,
    inmate_id: &i32,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<(), Error> {
    // Could do bulk insert here: https://github.com/launchbadge/sqlx/blob/main/FAQ.md#how-can-i-bind-an-array-to-a-values-clause-how-can-i-do-bulk-inserts
    // But, there is a low amount of bonds per inmate; therefores, its probably overengineering
    sqlx::query!(
        r#"
        INSERT INTO charge
            (inmate_id, description, grade, offense_date)
        VALUES
            ($1, $2, $3, $4)
        "#,
        inmate_id,
        charge.description,
        charge.grade.to_string(),
        charge.offense_date
    )
    .execute(&mut **transaction)
    .await?;

    trace!("Charge serialized: {:#?}", charge);
    Ok(())
}

async fn serialize_alias(
    alias: String,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i32, Error> {
    // two possible routes to the query -
    // -- insert alias, returning id
    // -- conflict on alias insert (duplicate), return existing alias id
    let res = sqlx::query!(
        r#"
        INSERT INTO alias
            (alias)
        VALUES
            ($1)
        ON CONFLICT (alias) DO UPDATE
            SET alias = EXCLUDED.alias
        RETURNING id
        "#,
        alias
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(res.id)
}

async fn serialize_profile(
    profile: InmateProfile,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    aws_s3_client: &Option<S3Client>,
) -> Result<i32, Error> {
    //TODO: Can this have compile time checks with pgvectgor extension? It doesn't seem possible
    //currently.

    // Pre-allocate the s3 url for the image
    let s3_img_url = if profile.img_blob.is_some() {
        profile.get_hash_on_core_attributes()
    } else {
        "".to_string()
    };

    let row = sqlx::query(
        r#"
        INSERT INTO inmate
        (
            first_name, middle_name, last_name, affix, permanent_id,
            sex, dob, arresting_agency, booking_date, booking_number,
            height, weight, race, eye_color, img_url, scil_sysid, embedding
        )
        VALUES
        (
            $1, $2, $3, $4, $5,
            $6, $7::date, $8, $9::timestamptz, $10,
            $11, $12, $13, $14, $15, $16, $17
        )
        RETURNING id
        "#,
    )
    .bind(profile.first_name)
    .bind(profile.middle_name)
    .bind(profile.last_name)
    .bind(profile.affix)
    .bind(profile.perm_id)
    .bind(profile.sex)
    .bind(profile.dob)
    .bind(profile.arrest_agency)
    .bind(profile.booking_date_iso8601)
    .bind(profile.booking_number)
    .bind(profile.height)
    .bind(profile.weight)
    .bind(profile.race)
    .bind(profile.eye_color)
    .bind(s3_img_url)
    .bind(profile.scil_sys_id)
    .bind(profile.embedding)
    .fetch_one(&mut **transaction)
    .await?;

    let inmate_id = row
        .try_get::<i32, _>("id")
        .expect("Expect inmate id to be present in profile serialization.");
    debug!(
        "Basic inmate data serialized to inmate table. Inmate ID: {}",
        inmate_id
    );

    // TODO: Now that we're confident we have a unique inmate, write img to s3
    /*
    if profile.img_blob.is_some() && aws_s3_client.is_some() {
        let img_url = s3_utils::upload_img_to_s3(
            aws_s3_client.as_ref().unwrap(),
            &s3_img_url,
            profile.img_blob.unwrap(),
        )
        .await?;
        sqlx::query!(
            r#"
            UPDATE inmate
            SET img_url = $1
            WHERE id = $2
            "#,
            img_url,
            inmate_id
        )
        .execute(&mut **transaction)
        .await?;
    }
    */

    // TODO: error handle failures on profile serialization that can be ignored? Letting
    // core profile data pass and ignoring the rest?
    for alias in profile.aliases.into_iter().flatten() {
        if alias.is_empty() {
            continue;
        }

        let alias_id = serialize_alias(alias, transaction).await?;

        sqlx::query!(
            r#"
            INSERT INTO inmate_alias
            (inmate_id, alias_id)
            VALUES
            ($1, $2)
            "#,
            inmate_id,
            alias_id
        )
        .execute(&mut **transaction)
        .await?;
    }
    debug!("Aliases serialized");

    sqlx::query!(
        r#"
        INSERT INTO img
            (inmate_id, img)
        VALUES
            ($1, $2)
        "#,
        inmate_id,
        profile.img_blob
    )
    .execute(&mut **transaction)
    .await?;
    debug!("Image serialized");

    Ok(inmate_id)
}
