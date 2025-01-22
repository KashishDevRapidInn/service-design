use diesel::sql_query;
use diesel_async::pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager};
use diesel_async::RunQueryDsl;
use diesel_async::{AsyncConnection, AsyncPgConnection};

pub type PgPool = Pool<AsyncPgConnection>;

/******************************************/
// Establishing Db Connection
/******************************************/
pub async fn establish_connection(database_url: &str) -> PgPool {
    let manager =
        AsyncDieselConnectionManager::<diesel_async::AsyncPgConnection>::new(database_url);

    // Build the pool
    let pool = Pool::builder(manager)
        .max_size(16)
        .build()
        .expect("Failed to create pool");

    pool
}

/******************************************/
// Creating new db for tests
/******************************************/
pub async fn create_database(database_name: &str, database_url: String) {
    let mut connection = AsyncPgConnection::establish(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    let create_db_query = format!(r#"CREATE DATABASE "{}";"#, database_name);
    sql_query(&create_db_query)
        .execute(&mut connection)
        .await
        .expect("Failed to create database");
    println!("Database '{}' created", database_name);
}

/******************************************/
// Dropping db code
/******************************************/
pub async fn drop_database(database_name: &str, default_db_url: String) {
    // Here I'm connecting to Postgres
    let mut connection = AsyncPgConnection::establish(&default_db_url)
        .await
        .expect("Failed to connect to the maintenance database");

    // My drop db logic wasn't working because I was trying to drop db which had active connection, so i need to delete my active connections
    let terminate_query = format!(
        r#"
        SELECT pg_terminate_backend(pid)
        FROM pg_stat_activity
        WHERE datname = '{}';
    "#,
        database_name
    );

    if let Err(e) = sql_query(&terminate_query).execute(&mut connection).await {
        eprintln!("Failed to terminate connections: {}", e);
        return;
    }

    // Dropping db
    let drop_query = format!(r#"DROP DATABASE IF EXISTS "{}";"#, database_name);

    if let Err(e) = sql_query(&drop_query).execute(&mut connection).await {
        eprintln!("Failed to drop database: {}", e);
    } else {
        println!("Database '{}' dropped successfully.", database_name);
    }
}
