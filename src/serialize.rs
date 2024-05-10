use sqlx::postgres::PgPool;

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
