use crate::diesel::{Connection, RunQueryDsl};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, Pool},
    sql_query,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;

use crate::DBPool;

pub struct TestContext {
    conn: PgConnection,
    base_url: String,
    db_name: String,
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/");

impl TestContext {
    pub fn new(db_name: &str) -> Self {
        let default = |_| "postgres://localhost".to_string();
        let base_url = env::var("TEST_DATABASE_URL").unwrap_or_else(default);
        println!("Creating test database `{}` in `{}` ...", db_name, base_url);
        println!("Note: please set `TEST_DATABASE_URL` to change this behavior.\n");

        let database_url = format!("{}/postgres", base_url);
        let mut conn = PgConnection::establish(&database_url).expect("Could not connect to database");

        // Create database
        sql_query(format!("CREATE DATABASE {}", db_name).as_str())
            .execute(&mut conn)
            .expect("Failed to create database");

        // Migation
        let conn_migrations = &mut PgConnection::establish(&format!("{}/{}", base_url, db_name))
            .unwrap_or_else(|_| panic!("Could not connect to database {}", db_name));
        conn_migrations
            .run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        Self {
            conn,
            base_url,
            db_name: db_name.to_string(),
        }
    }

    pub fn get_conn(&self) -> PgConnection {
        let database_url = format!("{}/{}", self.base_url, self.db_name);

        PgConnection::establish(&database_url).expect("Could not connect to database")
    }

    pub fn get_pool(&self) -> DBPool {
        let database_url = format!("{}/{}", self.base_url, self.db_name);
        let manager = ConnectionManager::<PgConnection>::new(database_url);

        Pool::builder()
            .build(manager)
            .expect("Failed to create connection pool")
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        println!("Dropping test database {}", self.db_name);

        let conn = &mut self.conn;

        // Postgres will refuse to delete a database
        // if there is any connected user
        sql_query(format!(
            "SELECT pg_terminate_backend(pid)
                FROM pg_stat_activity
                WHERE datname = '{}';",
            self.db_name
        ))
        .execute(conn)
        .unwrap();

        // Drop the database
        sql_query(format!("DROP DATABASE {}", self.db_name).as_str())
            .execute(conn)
            .unwrap_or_else(|_| panic!("Couldn't drop database {}", self.db_name));
    }
}
