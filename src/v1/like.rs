use actix_web::{delete, get, post, web, HttpResponse};
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::result::Error;
use diesel::{ExpressionMethods, Insertable, QueryDsl, Queryable, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

use crate::response::{ErrorResp, ListResp, Message};
use crate::schema::*;
use crate::{DBPool, DBPooledConnection};

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
#[table_name = "likes"]
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

pub fn list_likes(conn: &DBPooledConnection, _glossary_id: Uuid) -> Result<Vec<Like>, Error> {
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
    conn: &DBPooledConnection,
    _glossary_id: Uuid,
    _who: Option<String>,
) -> Result<Like, ErrorResp> {
    use crate::schema::likes::dsl::*;

    let like = Like::new(_who);

    let _ = diesel::insert_into(likes)
        .values(&like.to_like_db(_glossary_id))
        .execute(conn);

    Ok(like)
}

pub fn delete_one_like(
    conn: &DBPooledConnection,
    _glossary_id: Uuid,
    _like_id: Option<Uuid>,
) -> Result<(), ErrorResp> {
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

    let res = diesel::delete(likes.filter(id.eq(like_id))).execute(conn);
    match res {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// List likes for a glossary id
#[get("/glossary/{glossary_id}/likes")]
pub async fn list(
    web::Path(path): web::Path<(String,)>,
    pool: web::Data<DBPool>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&path.0).expect("could not parse glossary id");
    let likes = web::block(move || list_likes(&conn, glossary_id)).await;

    match likes {
        Ok(lks) => Ok(HttpResponse::Ok().json(Likes::from(&lks))),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Add one like to a glossary `/glossary/{id}/likes`
#[post("/glossary/{glossary_id}/likes")]
pub async fn plus_one(
    web::Path(path): web::Path<(String,)>,
    pool: web::Data<DBPool>,
    req: web::HttpRequest,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());

    let glossary_id = Uuid::from_str(&path.0).expect("could not parse glossary id");
    let like = web::block(move || create_like(&conn, glossary_id, who)).await;

    match like {
        Ok(like) => Ok(HttpResponse::Ok().json(like)),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Delete one like from a glossary `/glossary/{glossary_id}/likes`
#[delete("/glossary/{glossary_id}/likes")]
pub async fn minus_one(
    web::Path((glossary_id,)): web::Path<(String,)>,
    pool: web::Data<DBPool>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&glossary_id).expect("could not parse glossary id");

    match web::block(move || delete_one_like(&conn, glossary_id, None)).await {
        Ok(_) => Ok(HttpResponse::Ok().json(Message::new("ok"))),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{BodyTest, TestContext};
    use crate::v1::glossary::GlossaryDB;
    use actix_web::{test, App};
    use chrono::Utc;
    use uuid::Uuid;

    // Insert a glossary into database. Using the list likes to get the list of likes. The count
    // should be 0.
    #[actix_rt::test]
    async fn test_list_like_zero() {
        use crate::schema::glossary;

        let ctx = TestContext::new("test_create_like");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

        let glossary_id = Uuid::new_v4();
        let item_1 = GlossaryDB {
            id: glossary_id,
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert glossary item into database
        diesel::insert_into(glossary::table)
            .values(item_1)
            .execute(&conn)
            .expect("could not insert glossary");

        // Init api test server
        let mut app = test::init_service(
            App::new()
                .data(pool)
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Get the list of likes with GET /glossary/{id}/likes
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Count should be 0 because there is no like
        let body = resp.take_body().as_str().to_owned();
        let response: Likes = serde_json::from_str(&body).unwrap();
        assert_eq!(response.count, 0);
    }

    // Insert a glossary into database. Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 1.
    #[actix_rt::test]
    async fn test_list_like_with_one_like() {
        use crate::schema::glossary;

        let ctx = TestContext::new("test_list_like_with_one_like");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

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
            .execute(&conn)
            .expect("could not insert glossary");

        let mut app = test::init_service(
            App::new()
                .data(pool)
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create a like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        // Should be ok
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Count should be 1 because there is one like
        let body = resp.take_body().as_str().to_owned();
        let response: Likes = serde_json::from_str(&body).unwrap();
        assert_eq!(response.count, 1);
    }

    // Insert a glossary into database. Using the plus_one to create a like.
    // Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 2.
    #[actix_rt::test]
    async fn test_list_like_with_two_likes() {
        use crate::schema::glossary;

        let ctx = TestContext::new("test_list_like_with_two_likes");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

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
            .execute(&conn)
            .expect("could not insert glossary");

        let mut app = test::init_service(
            App::new()
                .data(pool)
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create the fist like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Create the 2nd like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();

        let mut resp = test::call_service(&mut app, req).await;
        let body = resp.take_body().as_str().to_owned();
        let response: Likes = serde_json::from_str(&body).unwrap();
        assert_eq!(response.count, 2);
    }

    // Insert a glossary into database.
    // Using the plus_one to create a like.
    // Using the list likes to get the list of likes. The count should be 1.
    // Using minus_one to delete the like.
    // Using the list likes to get the list of likes. The count should be 0.
    #[actix_rt::test]
    async fn test_list_like_with_one_like_and_minus_one() {
        use crate::schema::glossary;

        let ctx = TestContext::new("test_list_like_with_one_like_and_minus_one");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

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
            .execute(&conn)
            .expect("could not insert glossary");

        let mut app = test::init_service(
            App::new()
                .data(pool)
                .service(list)
                .service(plus_one)
                .service(minus_one),
        )
        .await;

        // Create a like using api POST /glossary/{glossary_id}/likes
        let req = test::TestRequest::post()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Count should be 1 because there is one like
        let body = resp.take_body().as_str().to_owned();
        let response: Likes = serde_json::from_str(&body).unwrap();
        assert_eq!(response.count, 1);

        // Delete the like using api DELETE /glossary/{glossary_id}/likes
        let req = test::TestRequest::delete()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Get the list of likes using GET /glossary/{glossary_id}/likes
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}/likes", glossary_id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;
        assert_eq!(resp.status().is_success(), true);

        // Count should be 0 because there is no like
        let body = resp.take_body().as_str().to_owned();
        let response: Likes = serde_json::from_str(&body).unwrap();
        assert_eq!(response.count, 0);
    }
}
