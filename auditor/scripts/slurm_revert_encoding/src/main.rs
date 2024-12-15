use dotenv::dotenv;
use sqlx::{PgPool, Postgres, Transaction};
use std::env;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/auditor".to_string());

    let pool = PgPool::connect(&database_url).await?;

    let mut transaction: Transaction<Postgres> = pool.begin().await?;

    let query = r#"
        UPDATE auditor_accounting
        SET meta = replace(meta::text, '%2F', '/')::jsonb;
    "#;

    let rows_affected = sqlx::query(query).execute(&mut *transaction).await;

    match rows_affected {
        Ok(result) => {
            println!("Rows affected: {}", result.rows_affected());

            transaction.commit().await?;
            Ok(())
        }
        Err(error) => {
            eprintln!("Error occurred: {:?}", error);

            transaction.rollback().await?;
            Err(error)
        }
    }
}
