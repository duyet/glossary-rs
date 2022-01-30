#[macro_use]
extern crate diesel;
extern crate actix_web_validator;
extern crate diesel_migrations;
extern crate dotenv;

pub mod response;
pub mod schema;
pub mod v1;

pub use diesel::pg::PgConnection;
pub use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

pub type DBPool = Pool<ConnectionManager<PgConnection>>;
pub type DBPooledConnection = PooledConnection<ConnectionManager<PgConnection>>;

pub const AUTHENTICATED_USER_HEADER: &str = "x-authenticated-user-email";
