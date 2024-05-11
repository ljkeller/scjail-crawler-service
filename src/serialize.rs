use sqlx::postgres::PgPool;

use crate::inmate::{InmateProfile, Record};
use crate::Error;

pub async fn create_dbs(pool: &PgPool) -> Result<(), Error> {
    sqlx::query_file!("queries/create_inmate.sql")
        .execute(pool)
        .await
        .expect("Expect create_inamate.sql to run");
    sqlx::query_file!("queries/create_alias.sql")
        .execute(pool)
        .await
        .expect("Expect create_alias.sql to run");
    sqlx::query_file!("queries/create_bond.sql")
        .execute(pool)
        .await
        .expect("Expect create_bond.sql to run");
    sqlx::query_file!("queries/create_charge.sql")
        .execute(pool)
        .await
        .expect("Expect create_charge.sql to run");
    sqlx::query_file!("queries/create_img.sql")
        .execute(pool)
        .await
        .expect("Expect create_img.sql to run");
    sqlx::query_file!("queries/create_inmate_alias.sql")
        .execute(pool)
        .await
        .expect("Expect create_inmate_alias.sql to run");

    Ok(())
}

pub async fn inmate_count(pool: &PgPool) -> Result<i64, Error> {
    let res = sqlx::query!("SELECT COUNT(*) FROM inmate")
        .fetch_one(pool)
        .await?;
    Ok(res
        .count
        .expect("Expect count to be present on on inmate count query"))
}

pub async fn serialize_record(record: Record, pool: &PgPool) -> Result<i32, Error> {
    let mut transaction = pool.begin().await?;
    let inserted_id = serialize_profile(record.profile, &mut transaction).await?;

    Ok(inserted_id)
}

async fn serialize_profile(
    profile: InmateProfile,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> Result<i32, Error> {
    // TODO: use query! macro for compile time verification
    let res = sqlx::query!(
        r#"
        INSERT INTO inmate
        (
            first_name, middle_name, last_name, affix, permanent_id,
            sex, dob, arresting_agency, booking_date, booking_number, 
            height, weight, race, eye_color, scil_sysid
        )
        VALUES
        (
            $1, $2, $3, $4, $5,
            $6, $7::date, $8, $9::timestamptz, $10,
            $11, $12, $13, $14, $15
        )
        RETURNING id
        "#,
        profile.first_name,
        profile.middle_name,
        profile.last_name,
        profile.affix,
        profile.perm_id,
        profile.sex,
        profile.dob as _, // TODO: avoid override and use NaiveDate
        profile.arrest_agency,
        profile.booking_date_iso8601 as _, // TODO: avoid override and use NaiveDateTime
        profile.booking_number,
        profile.height,
        profile.weight,
        profile.race,
        profile.eye_color,
        profile.scil_sys_id,
    )
    .fetch_one(&mut **transaction)
    .await?;

    Ok(res.id)
}
