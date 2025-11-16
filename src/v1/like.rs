use actix_web::{delete, get, post, web, HttpRequest, Responder, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    pg::PgConnection, result::Error, ExpressionMethods, Insertable, QueryDsl, Queryable,
    RunQueryDsl,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::{
    response::{ApiError, ErrorResp, ListResp, Message},
    schema::*,
    DBPool,
};

pub type Likes = ListResp<Like>;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Like {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub who: Option<String>,
}

impl Default for Like {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Like {
    pub fn new(who: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            who,
        }
    }

    pub fn to_like_db(&self, glossary_id: Uuid) -> LikeDB {
        LikeDB {
            id: Uuid::from_str(self.id.as_str()).unwrap(),
            created_at: self.created_at.naive_utc(),
            glossary_id,
            who: self.who.clone(),
        }
    }
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = likes)]
pub struct LikeDB {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub glossary_id: Uuid,
    pub who: Option<String>,
}

impl LikeDB {
    pub fn to_like(&self) -> Like {
        Like {
            id: self.id.to_string(),
            created_at: DateTime::<Utc>::from_utc(self.created_at, Utc),
            who: self.who.clone(),
        }
    }
}

pub fn list_likes(conn: &mut PgConnection, _glossary_id: Uuid) -> Result<Vec<Like>, Error> {
    use crate::schema::likes::dsl::*;

    match likes
        .filter(glossary_id.eq(_glossary_id))
        .order(created_at.desc())
        .load::<LikeDB>(conn)
    {
        Ok(lks) => Ok(lks.into_iter().map(|l| l.to_like()).collect()),
        Err(_) => Ok(vec![]),
    }
}

pub fn create_like(
    conn: &mut PgConnection,
    _glossary_id: Uuid,
    _who: Option<String>,
) -> Result<Like, Error> {
    use crate::schema::likes::dsl::*;

    let like = Like::new(_who);

    diesel::insert_into(likes)
        .values(&like.to_like_db(_glossary_id))
        .execute(conn)?;

    Ok(like)
}

pub fn delete_one_like(
    conn: &mut PgConnection,
    _glossary_id: Uuid,
    _like_id: Option<Uuid>,
) -> Result<(), Error> {
    use crate::schema::likes::dsl::*;

    let like: Option<Like> = if let Some(like_id) = _like_id {
        match likes.filter(id.eq(like_id)).load::<LikeDB>(conn) {
            Ok(lks) => lks.first().map(|v| v.to_like()),
            _ => None,
        }
    } else {
        match list_likes(conn, _glossary_id) {
            Ok(_likes) if !_likes.is_empty() => _likes.first().cloned(),
            _ => None,
        }
    };

    if like.is_none() {
        return Ok(());
    }

    let like_id = Uuid::from_str(like.unwrap().id.as_str()).unwrap();
    diesel::delete(likes.filter(id.eq(like_id))).execute(conn)?;
    Ok(())
}

/// List likes for a glossary id
#[get("/glossary/{glossary_id}/likes")]
pub async fn list(
    id: web::Path<String>,
    pool: web::Data<DBPool>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    let likes = web::block(move || list_likes(&mut conn, glossary_id)).await??;
    Ok(web::Json(Likes::from(&likes)))
}

/// Add one like to a glossary `/glossary/{id}/likes`
#[post("/glossary/{glossary_id}/likes")]
pub async fn plus_one(
    id: web::Path<String>,
    pool: web::Data<DBPool>,
    req: HttpRequest,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());

    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    let like = web::block(move || create_like(&mut conn, glossary_id, who)).await??;
    Ok(web::Json(like))
}

/// Delete one like from a glossary `/glossary/{glossary_id}/likes`
#[delete("/glossary/{glossary_id}/likes")]
pub async fn minus_one(
    id: web::Path<String>,
    pool: web::Data<DBPool>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    web::block(move || delete_one_like(&mut conn, glossary_id, None)).await??;
    Ok(web::Json(Message::new("ok")))
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use crate::v1::glossary::GlossaryDB;
    use actix_web::{http, test, App};
    use chrono::Utc;
    use uuid::Uuid;

    macro_rules! service_should_ok_and_return_json {
        ($app:expr, $req:expr) => {{
            let req = test::TestRequest::from($req).to_request();
            let resp = test::call_service(&$app, req).await;
            println!("Debug: Resp = {:?}", resp);

            assert!(resp.status().is_success());
            assert_eq!(
                resp.headers().get("content-type").unwrap(),
                "application/json"
            );

            resp
        }};
    }

    // Using the list likes to get the list of non-exist glossary.
    // The count should be 0.
    #[actix_rt::test]
    async fn list_like_non_exists_glossary() {
        let ctx = TestContext::new("list_like_non_exists_glossary");
        let pool = ctx.get_pool();

        let non_exist_id = Uuid::new_v4();

        // Init api test server
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Get the list of likes with GET /glossary/{id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", non_exist_id));
        let resp = service_should_ok_and_return_json!(app, req);

        // Count should be 0 because there is no like
        let response: Likes = test::read_body_json(resp).await;
        assert_eq!(response.count, 0);
    }

    // Using the list likes to get the list of invalid glossary id.
    // Should return 400 BAD REQUEST
    #[actix_rt::test]
    async fn list_like_invalid_glossary() {
        let ctx = TestContext::new("list_like_invalid_glossary");
        let pool = ctx.get_pool();

        let invalid_glossary_id = "abc1234";

        // Init api test server
        let app = test::init_service(App::new().app_data(web::Data::new(pool)).service(list)).await;

        // Get the list of likes with GET /glossary/{id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", invalid_glossary_id));
        let resp = test::call_service(&app, req.to_request()).await;

        // Should return 400 BAD REQUEST
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    // Post new like to invalid glossary id.
    // Should return 400 BAD REQUEST
    #[actix_rt::test]
    async fn add_like_invalid_glossary() {
        let ctx = TestContext::new("add_like_invalid_glossary");
        let pool = ctx.get_pool();

        let invalid_glossary_id = "abc1234";

        // Init api test server
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one),
        )
        .await;

        // Get the list of likes with POST /glossary/{id}/likes
        let req =
            test::TestRequest::post().uri(&format!("/glossary/{}/likes", invalid_glossary_id));
        let resp = test::call_service(&app, req.to_request()).await;

        // Should return 400 BAD REQUEST
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }

    // Insert a glossary into database.
    // Using the list likes to get the list of likes. The count
    // should be 0.
    #[actix_rt::test]
    async fn list_like_empty() {
        use crate::schema::glossary;

        let ctx = TestContext::new("list_like_empty");
        let pool = ctx.get_pool();
        let conn = &mut pool.get().expect("could not get db connection from pool");

        let glossary_id = Uuid::new_v4();
        let item = GlossaryDB {
            id: glossary_id,
            term: "test_term".to_string(),
            definition: "test_definition".to_string(),
            revision: 1,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert glossary item into database
        diesel::insert_into(glossary::table)
            .values(item)
            .execute(conn)
            .expect("could not insert glossary");

        // Init api test server
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Get the list of likes with GET /glossary/{id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", glossary_id));
        let resp = service_should_ok_and_return_json!(app, req);

        // Count should be 0 because there is no like
        let response: Likes = test::read_body_json(resp).await;
        assert_eq!(response.count, 0);
    }

    // Insert a glossary into database. Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 1.
    #[actix_rt::test]
    async fn one_like() {
        use crate::schema::glossary;

        let ctx = TestContext::new("one_like");
        let pool = ctx.get_pool();
        let conn = &mut pool.get().expect("could not get db connection from pool");
        let glossary_id = Uuid::new_v4();

        let item = GlossaryDB {
            id: glossary_id,
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert to glossaries
        diesel::insert_into(glossary::table)
            .values(item)
            .execute(conn)
            .expect("could not insert glossary");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create a like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .set_json(b"{}");
        let _ = service_should_ok_and_return_json!(app, req);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", glossary_id));
        let resp = service_should_ok_and_return_json!(app, req);

        // Count should be 1 because there is one like
        let response: Likes = test::read_body_json(resp).await;
        assert_eq!(response.count, 1);
    }

    // Using the plus_one to create a like for non-existent glossary
    // Should return 409 CONFLICT (foreign key constraint violation)
    #[actix_rt::test]
    async fn one_like_non_exists_glossary() {
        let ctx = TestContext::new("one_like_non_exists_glossary");
        let pool = ctx.get_pool();

        let non_exists_glossary_id = Uuid::new_v4();

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create a like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", non_exists_glossary_id))
            .set_json(b"{}")
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Foreign key violation returns 409 CONFLICT
        assert_eq!(resp.status(), http::StatusCode::CONFLICT);
    }

    // Insert a glossary into database. Using the plus_one to create a like.
    // Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 2.
    #[actix_rt::test]
    async fn like_two_times() {
        use crate::schema::glossary;

        let ctx = TestContext::new("like_two_times");
        let pool = ctx.get_pool();
        let conn = &mut pool.get().expect("could not get db connection from pool");

        let glossary_id = Uuid::new_v4();

        let item_1 = GlossaryDB {
            id: glossary_id,
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert two glossaries
        diesel::insert_into(glossary::table)
            .values(item_1)
            .execute(conn)
            .expect("could not insert glossary");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create the fist like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post().uri(&format!("/glossary/{}/likes", glossary_id));
        let _ = service_should_ok_and_return_json!(app, req);

        // Create the 2nd like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post().uri(&format!("/glossary/{}/likes", glossary_id));
        let _ = service_should_ok_and_return_json!(app, req);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", glossary_id));
        let resp = service_should_ok_and_return_json!(app, req);
        let response: Likes = test::read_body_json(resp).await;
        assert_eq!(response.count, 2);
    }

    // Insert a glossary into database.
    // Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 1.
    // Using minus_one to delete the like.
    // Using the list likes to get the list of likes. The count should be 0.
    #[actix_rt::test]
    async fn like_then_unlike() {
        use crate::schema::glossary;

        let ctx = TestContext::new("like_then_unlike");
        let pool = ctx.get_pool();
        let conn = &mut pool.get().expect("could not get db connection from pool");

        let glossary_id = Uuid::new_v4();

        let item_1 = GlossaryDB {
            id: glossary_id,
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert two glossaries
        diesel::insert_into(glossary::table)
            .values(item_1)
            .execute(conn)
            .expect("could not insert glossary");

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(pool))
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create a like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post().uri(&format!("/glossary/{}/likes", glossary_id));
        let _ = service_should_ok_and_return_json!(app, req);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", glossary_id));
        let resp = service_should_ok_and_return_json!(app, req);
        // Count should be 1 because there is one like
        let likes: Likes = test::read_body_json(resp).await;
        assert_eq!(likes.count, 1);

        // Delete the like using api DELETE /glossary/{glossary_id}/likes
        let req = test::TestRequest::delete().uri(&format!("/glossary/{}/likes", glossary_id));
        let _ = service_should_ok_and_return_json!(app, req);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get().uri(&format!("/glossary/{}/likes", glossary_id));
        let resp = service_should_ok_and_return_json!(app, req);
        // Count should be 0 because there is no like
        let likes: Likes = test::read_body_json(resp).await;
        assert_eq!(likes.count, 0);
    }
}
