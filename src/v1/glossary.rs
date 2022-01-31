use actix_web::{delete, get, post, put, web, HttpRequest, HttpResponse};
use actix_web_validator::Json;
use ammonia::clean;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::result::Error;
use diesel::{ExpressionMethods, GroupByDsl, QueryDsl, RunQueryDsl};
use diesel::{Insertable, Queryable};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use uuid::Uuid;
use validator::Validate;

use super::glossary_history::create_glossary_history;
use super::like::{list_likes, Like};
use crate::response::{ErrorResp, ListResp, Message};
use crate::schema::*;
use crate::{DBPool, DBPooledConnection};

pub type Glossaries = ListResp<Glossary>;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Glossary {
    pub id: String,
    pub term: String,
    pub definition: String,
    pub revision: i32,
    pub likes: Vec<Like>,
    pub likes_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Glossary {
    pub fn new(term: String, definition: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            term,
            definition,
            revision: 0,
            likes: vec![],
            likes_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn to_glossary_db(&self) -> GlossaryDB {
        GlossaryDB {
            id: Uuid::new_v4(),
            term: self.term.clone(),
            definition: self.definition.clone(),
            revision: self.revision,
            created_at: self.created_at.naive_utc(),
            updated_at: self.updated_at.naive_utc(),
        }
    }

    pub fn add_likes(&self, likes: Vec<Like>) -> Self {
        let likes_count = self.likes_count + likes.len() as i32;
        Self {
            likes,
            likes_count,
            ..self.clone()
        }
    }
}

#[derive(Queryable, Insertable)]
#[table_name = "glossary"]
pub struct GlossaryDB {
    pub id: Uuid,
    pub term: String,
    pub definition: String,
    pub revision: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl GlossaryDB {
    pub fn to_glossary(&self) -> Glossary {
        Glossary {
            id: self.id.to_string(),
            term: self.term.clone(),
            definition: self.definition.clone(),
            revision: self.revision,
            likes: vec![],
            likes_count: 0,
            created_at: DateTime::<Utc>::from_utc(self.created_at, Utc),
            updated_at: DateTime::<Utc>::from_utc(self.updated_at, Utc),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct GlossaryRequest {
    #[validate(required, length(min = 1, max = 255))]
    #[serde(deserialize_with = "cleanup_string")]
    pub term: Option<String>,
    #[validate(required)]
    #[serde(deserialize_with = "cleanup_string")]
    pub definition: Option<String>,
}

fn cleanup_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    let s = clean(s.trim());
    Ok(Some(s))
}

impl GlossaryRequest {
    pub fn to_glossary(&self) -> Option<Glossary> {
        match (&self.term, &self.definition) {
            (Some(term), Some(definition)) => {
                Some(Glossary::new(term.to_string(), definition.to_string()))
            }
            (Some(term), _) => Some(Glossary::new(term.to_string(), "".to_string())),
            _ => None,
        }
    }
}

fn list_glossary(conn: &DBPooledConnection) -> Result<Vec<GlossaryDB>, Error> {
    use crate::schema::glossary::dsl::*;

    glossary.order(term.asc()).load(conn)
}

fn create_glossary(
    conn: &DBPooledConnection,
    value: Json<GlossaryRequest>,
    who: Option<String>,
) -> Result<GlossaryDB, Error> {
    use crate::schema::glossary::dsl::*;

    let _glossary = value.into_inner().to_glossary().unwrap();

    let created = diesel::insert_into(glossary)
        .values(_glossary.to_glossary_db())
        .returning((id, term, definition, revision, created_at, updated_at))
        .get_result::<GlossaryDB>(conn)?;

    create_glossary_history(
        conn,
        created.term.to_string(),
        created.definition.to_string(),
        who,
        created.revision,
        created.id,
    );

    Ok(created)
}

fn get_glossary(conn: &DBPooledConnection, _id: Uuid) -> Result<GlossaryDB, Error> {
    use crate::schema::glossary::dsl::*;

    glossary.filter(id.eq(_id)).first::<GlossaryDB>(conn)
}

fn update_glossary(
    conn: &DBPooledConnection,
    _id: Uuid,
    value: Glossary,
    who: Option<String>,
) -> Result<GlossaryDB, Error> {
    use crate::schema::glossary::dsl::*;

    let updated = diesel::update(glossary.find(_id))
        .set((
            term.eq(value.term),
            definition.eq(value.definition),
            revision.eq(revision + 1),
            updated_at.eq(Utc::now().naive_utc()),
        ))
        .returning((id, term, definition, revision, created_at, updated_at))
        .get_result::<GlossaryDB>(conn)?;

    create_glossary_history(
        conn,
        updated.term.to_string(),
        updated.definition.to_string(),
        who,
        updated.revision,
        updated.id,
    );

    Ok(updated)
}

fn delete_glossary(conn: &DBPooledConnection, _id: Uuid) -> Result<usize, Error> {
    use crate::schema::glossary::dsl::*;

    // Need to delete foreign table first: glossary_history
    match diesel::delete(
        glossary_history::table.filter(glossary_history::columns::glossary_id.eq(_id)),
    )
    .execute(conn)
    {
        Ok(count) => {
            if count > 0 {
                diesel::delete(glossary.find(_id)).execute(conn)
            } else {
                Ok(0)
            }
        }
        Err(e) => Err(e),
    }
}

fn list_popular_glossary(
    conn: &DBPooledConnection,
    limit: Option<u8>,
) -> Result<Vec<Glossary>, Error> {
    use diesel::dsl;
    use diesel::pg::expression::dsl::any;

    let limit = limit.unwrap_or(10);

    // Most likes glossaries
    let most_glossary_id_by_count = likes::table
        .select(likes::columns::glossary_id)
        .order(dsl::count_star().desc())
        .group_by(likes::columns::glossary_id)
        .limit(limit as i64)
        .load::<Uuid>(conn)?;

    // Get glossaries in the list
    let glossaries = glossary::table
        .filter(glossary::columns::id.eq(any(most_glossary_id_by_count)))
        .load::<GlossaryDB>(conn)
        .unwrap()
        .into_iter()
        .map(|a| a.to_glossary())
        .collect();

    Ok(glossaries)
}

pub type GroupedGlossary = std::collections::HashMap<String, Vec<Glossary>>;

/// List all glossaries
#[get("/glossary")]
pub async fn list(pool: web::Data<DBPool>) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    // Diesel does not support tokio (the asynchronous engine behind Actix),
    // so we have to run it in separate threads using the web::block
    let glossaries = web::block(move || list_glossary(&conn)).await;

    let conn = pool.get().expect("could not get db connection from pool");

    match glossaries {
        Ok(glossaries) => {
            let mut glossaries_by_alphabet: HashMap<String, Vec<Glossary>> = HashMap::new();

            glossaries.into_iter().for_each(|a| {
                let likes = list_likes(&conn, Uuid::from_str(&a.id.to_string()).unwrap()).unwrap();

                let character = a.term.chars().next().unwrap().to_uppercase();
                let b = glossaries_by_alphabet
                    .entry(character.to_string())
                    .or_insert_with(Vec::new);
                b.push(a.to_glossary().add_likes(likes));
            });

            Ok(HttpResponse::Ok().json(glossaries_by_alphabet as GroupedGlossary))
        }
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Find a glossary by id
#[get("/glossary/{id}")]
pub async fn get(
    pool: web::Data<DBPool>,
    web::Path(id): web::Path<String>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    match web::block(move || get_glossary(&conn, Uuid::from_str(id.as_str()).unwrap())).await {
        Ok(g) => Ok(HttpResponse::Ok().json(g.to_glossary())),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Update a glossary by id
#[put("/glossary/{id}")]
pub async fn update(
    pool: web::Data<DBPool>,
    web::Path(id): web::Path<String>,
    Json(value): Json<GlossaryRequest>,
    req: HttpRequest,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());

    match web::block(move || {
        update_glossary(
            &conn,
            Uuid::from_str(id.as_str()).unwrap(),
            value.to_glossary().unwrap(),
            who,
        )
    })
    .await
    {
        Ok(g) => Ok(HttpResponse::Ok().json(g.to_glossary())),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Delete a glossary by id
#[delete("/glossary/{id}")]
pub async fn delete(
    pool: web::Data<DBPool>,
    web::Path(id): web::Path<String>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    match web::block(move || delete_glossary(&conn, Uuid::from_str(id.as_str()).unwrap())).await {
        Ok(_) => Ok(HttpResponse::Ok().json(Message::new("deleted"))),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

/// Create a new glossary
#[post("/glossary")]
pub async fn create(
    json: Json<GlossaryRequest>,
    req: HttpRequest,
    pool: web::Data<DBPool>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());

    match web::block(move || create_glossary(&conn, json, who)).await {
        Ok(result) => Ok(HttpResponse::Ok().json(result.to_glossary())),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

#[derive(Deserialize)]
pub struct PopularQuery {
    pub limit: Option<u8>,
}

/// List popular glossaries
#[get("/glossary-popular")]
pub async fn list_popular(
    pool: web::Data<DBPool>,
    query: web::Query<PopularQuery>,
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let glossaries = web::block(move || {
        let limit = query.limit;
        list_popular_glossary(&conn, limit)
    })
    .await;

    match glossaries {
        Ok(glossaries) => Ok(HttpResponse::Ok().json(glossaries)),
        Err(e) => Err(ErrorResp::new(&e.to_string())),
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{BodyTest, TestContext};
    use crate::v1::like::create_like;
    use actix_web::{test, App};

    // Insert a glossary into database.
    // Than, using API to get list of glossaries
    #[actix_rt::test]
    async fn test_list_glossary() {
        let ctx = TestContext::new("test_list_glossary");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

        let item_1 = GlossaryDB {
            id: Uuid::new_v4(),
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };
        let item_2 = GlossaryDB {
            id: Uuid::new_v4(),
            term: "test_term_2".to_string(),
            revision: 1,
            definition: "test_definition_2".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert two glossaries
        diesel::insert_into(glossary::table)
            .values(item_1)
            .execute(&conn)
            .expect("could not insert glossary");
        diesel::insert_into(glossary::table)
            .values(item_2)
            .execute(&conn)
            .expect("could not insert glossary");

        let mut app = test::init_service(App::new().data(pool).service(list)).await;
        let req = test::TestRequest::get().uri("/glossary").to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // The response should be:
        // {
        //   "T": [
        //      { "term": "test_term_1", "definition": "test_definition_1", ... },
        //      { "term": "test_term_2", "definition": "test_definition_2", ... },
        //   ]
        // }
        let body = resp.take_body().as_str().to_owned();
        println!("{}", body);
        let response: GroupedGlossary = serde_json::from_str(&body).unwrap();
        assert_eq!(response.keys().len(), 1);
        let values = response.get("T").unwrap();
        assert_eq!(values.len(), 2);
    }

    // Direct into data to Database.
    // Than, using API to get glossary
    #[actix_rt::test]
    async fn test_get_glossary() {
        let ctx = TestContext::new("test_get_glossary");
        let pool = ctx.get_pool();
        let conn = pool.get().expect("could not get db connection from pool");

        let item_id = Uuid::new_v4();
        let api_url = format!("/glossary/{}", item_id);

        let item_1 = GlossaryDB {
            id: item_id,
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

        let mut app = test::init_service(App::new().data(pool).service(get)).await;
        let req = test::TestRequest::get().uri(&api_url).to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Body JSON should contains the term and definition
        let body = resp.take_body().as_str().to_owned();
        let response: Glossary = serde_json::from_str(&body).unwrap();
        assert_eq!(response.term, "test_term_1");
        assert_eq!(response.definition, "test_definition_1");
        assert_eq!(response.revision, 1);
    }

    // Using API to create glossary
    #[actix_rt::test]
    async fn test_create_glossary() {
        let ctx = TestContext::new("test_create_glossary");
        let pool = ctx.get_pool();

        let mut app = test::init_service(App::new().data(pool).service(create)).await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Body JSON should contains the term and definition
        let body = resp.take_body().as_str().to_owned();
        let response: Glossary = serde_json::from_str(&body).unwrap();
        assert_eq!(response.term, "test_term_1");
        assert_eq!(response.definition, "test_definition_1");
    }

    // Using API to create glossary. Than, using API to get glossary.
    #[actix_rt::test]
    async fn test_create_than_get_glossary() {
        let ctx = TestContext::new("test_create_than_get_glossary");
        let pool = ctx.get_pool();

        let mut app = test::init_service(App::new().data(pool).service(create).service(get)).await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        let body = resp.take_body().as_str().to_owned();
        let response_of_create: Glossary = serde_json::from_str(&body).unwrap();

        // Get glossary from api
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}", response_of_create.id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = resp.take_body().as_str().to_owned();
        let response_of_get: Glossary = serde_json::from_str(&body).unwrap();

        // Response of API create should be equal to response of API get
        assert_eq!(response_of_get, response_of_create);
    }

    // Using API to create glossary. Than, using API to update glossary.
    #[actix_rt::test]
    async fn test_create_than_update_glossary() {
        let ctx = TestContext::new("test_create_than_update_glossary");
        let pool = ctx.get_pool();

        let mut app =
            test::init_service(App::new().data(pool).service(create).service(update)).await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        let body = resp.take_body().as_str().to_owned();
        let response_of_create: Glossary = serde_json::from_str(&body).unwrap();

        // Verify that insert correctly
        assert_eq!(response_of_create.term, "test_term_1");
        assert_eq!(response_of_create.definition, "test_definition_1");
        assert_eq!(response_of_create.revision, 0);

        // Update glossary
        let req = test::TestRequest::put()
            .uri(&format!("/glossary/{}", response_of_create.id))
            .set_json(&GlossaryRequest {
                term: Some("test_term_1_updated".to_string()),
                definition: Some("test_definition_1_updated".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Body JSON should contains the term and definition
        let body = resp.take_body().as_str().to_owned();
        let response_of_update: Glossary = serde_json::from_str(&body).unwrap();
        assert_eq!(response_of_update.term, "test_term_1_updated");
        assert_eq!(response_of_update.definition, "test_definition_1_updated");
        assert_eq!(response_of_update.revision, 1);
    }

    // Using API to create glossary. Than, using API to delete glossary.
    #[actix_rt::test]
    async fn test_create_than_delete_glossary() {
        let ctx = TestContext::new("test_create_than_delete_glossary");
        let pool = ctx.get_pool();

        let mut app = test::init_service(
            App::new()
                .data(pool)
                .service(create)
                .service(get)
                .service(delete),
        )
        .await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        let body = resp.take_body().as_str().to_owned();
        let response_of_create: Glossary = serde_json::from_str(&body).unwrap();

        // Verify that insert correctly
        assert_eq!(response_of_create.term, "test_term_1");
        assert_eq!(response_of_create.definition, "test_definition_1");
        assert_eq!(response_of_create.revision, 0);

        // Delete glossary
        let req = test::TestRequest::delete()
            .uri(&format!("/glossary/{}", response_of_create.id))
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Using to debug if the delete is not working
        let body = resp.take_body().as_str().to_owned();
        println!("{}", body);

        // Response should be OK
        assert!(resp.status().is_success());
        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        // Response should be: {"message": "deleted"}
        let response_of_delete: Message = serde_json::from_str(&body).unwrap();
        assert_eq!(response_of_delete.message, "deleted");

        // For sure, we get the deleted glossary via API again
        let req = test::TestRequest::get()
            .uri(&format!("/glossary/{}", response_of_create.id))
            .to_request();
        let resp = test::call_service(&mut app, req).await;

        println!("{:?}", resp.status());

        // Response should be NOT_FOUND
        assert!(resp.status().is_client_error());
    }

    // Fast test list popular glossaries
    // By default, the list should be empty
    #[actix_rt::test]
    async fn test_list_popular_glossaries_empty() {
        let ctx = TestContext::new("test_list_popular_glossaries");
        let pool = ctx.get_pool();

        let mut app = test::init_service(App::new().data(pool).service(list_popular)).await;
        let req = test::TestRequest::get()
            .uri("/glossary-popular")
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = resp.take_body().as_str().to_owned();
        let response_of_list_popular: Vec<Glossary> = serde_json::from_str(&body).unwrap();

        // By default: the list popular should be empty if there is not likes on any glossary
        assert!(response_of_list_popular.is_empty());
    }

    // Test list popular glossaries with inserted glossaries
    #[actix_rt::test]
    async fn test_list_popular_glossaries_with_inserted_glossaries() {
        let ctx = TestContext::new("test_list_popular_glossaries_with_inserted_glossaries");
        let pool = ctx.get_pool();
        let conn = pool.get().unwrap();

        let mut app =
            test::init_service(App::new().data(pool).service(create).service(list_popular)).await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            })
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Verify that insert correctly
        let body = resp.take_body().as_str().to_owned();
        let response_of_create: Glossary = serde_json::from_str(&body).unwrap();
        assert_eq!(response_of_create.term, "test_term_1");
        assert_eq!(response_of_create.definition, "test_definition_1");
        assert_eq!(response_of_create.revision, 0);

        let glossary_id = Uuid::from_str(&response_of_create.id).unwrap();
        let _ = create_like(&conn, glossary_id, None);

        // Get the list popular
        let req = test::TestRequest::get()
            .uri("/glossary-popular")
            .to_request();
        let mut resp = test::call_service(&mut app, req).await;

        // Response should be OK
        assert!(resp.status().is_success());

        // Response should be application/json
        assert_eq!(
            resp.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = resp.take_body().as_str().to_owned();
        let response_of_list_popular: Vec<Glossary> = serde_json::from_str(&body).unwrap();
        assert_eq!(response_of_list_popular.len(), 1);
    }
}
