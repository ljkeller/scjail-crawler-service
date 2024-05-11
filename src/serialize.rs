use sqlx::postgres::PgPool;

use crate::inmate::Record;
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

pub async fn serialize_record(record: Record, pool: &PgPool) -> Result<i32, Error> {
    // TODO: use query! macro for compile time verification
    let res = sqlx::query(
        r#"
        INSERT INTO inmate
        (
            first_name, middle_name, last_name, affix, permanent_id,
            sex, dob, arresting_agency, booking_date, booking_number, 
            height, weight, race, eye_color, img_url, scil_sysid
        )
        VALUES
        (
            $1, $2, $3, $4, $5,
            $6, CAST($7 AS DATE), $8, $9::timestamptz, $10,
            $11, $12, $13, $14, $15, $16
        )
        RETURNING id
        "#,
    )
    .bind(record.profile.first_name)
    .bind(record.profile.middle_name)
    .bind(record.profile.last_name)
    .bind(record.profile.affix)
    .bind(record.profile.perm_id)
    .bind(record.profile.sex)
    .bind(record.profile.dob)
    .bind(record.profile.arrest_agency)
    .bind(record.profile.booking_date_iso8601)
    .bind(record.profile.booking_number)
    .bind(record.profile.height)
    .bind(record.profile.weight)
    .bind(record.profile.race)
    .bind(record.profile.eye_color)
    .bind(String::from("")) //TODO: Add img_ur)
    .bind(record.profile.scil_sys_id)
    .execute(pool)
    // .fetch_one(pool)
    .await?;

    dbg!(res);

    Ok(0)
}
