#[macro_use]
extern crate diesel_migrations;

use actix_cors::Cors;
use actix_web::{get, web, HttpResponse, Responder};
use actix_web::{middleware, App, HttpServer};
use diesel::pg::PgConnection;
use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::embed_migrations;
use dotenv::dotenv;
use log::info;
use std::env;

use glossary::response;
use glossary::v1;

#[get("/")]
pub async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/ping")]
pub async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    env_logger::init();
    dotenv().ok();

    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let listen = format!("{}:{}", host, port);

    // set up database connection pool
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    info!("Connecting to database: {}", database_url);
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::builder()
        .build(manager)
        .expect("Failed to create connection pool");

    // Migration schema
    embed_migrations!("./migrations");
    let conn = pool.get().expect("could not get db connection from pool");
    embedded_migrations::run_with_output(&conn, &mut std::io::stdout()).unwrap();

    let server = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_header()
            .allow_any_origin()
            .allow_any_method();

        App::new()
            .data(pool.clone())
            .app_data(web::JsonConfig::default().error_handler(response::json_error_handler))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(index)
            .service(ping)
            .service(
                web::scope("/api/v1/")
                    .service(v1::glossary::list)
                    .service(v1::glossary::list_popular)
                    .service(v1::glossary::get)
                    .service(v1::glossary::update)
                    .service(v1::glossary::delete)
                    .service(v1::glossary::create)
                    .service(v1::like::list)
                    .service(v1::like::plus_one)
                    .service(v1::like::minus_one),
            )
    })
    .bind(listen.to_string())?
    .run();

    info!("Server running at http://{}", listen);

    server.await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body::{Body, ResponseBody};
    use actix_web::test;

    trait BodyTest {
        fn as_str(&self) -> &str;
    }

    impl BodyTest for ResponseBody<Body> {
        fn as_str(&self) -> &str {
            match self {
                ResponseBody::Body(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
                ResponseBody::Other(ref b) => match b {
                    Body::Bytes(ref by) => std::str::from_utf8(by).unwrap(),
                    _ => panic!(),
                },
            }
        }
    }

    #[actix_rt::test]
    async fn test_index_get() {
        let mut app = test::init_service(App::new().service(index)).await;
        let req = test::TestRequest::with_header("content-type", "text/plain").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_rt::test]
    async fn test_ping() {
        let mut app = test::init_service(App::new().service(ping)).await;
        let req = test::TestRequest::get().uri("/ping").to_request();
        let mut resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
        assert_eq!(resp.take_body().as_str(), "pong");
    }
}
