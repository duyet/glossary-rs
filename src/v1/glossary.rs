use actix_web::{delete, get, post, put, web, HttpRequest, Responder};
use actix_web_validator::Json;
use ammonia::clean;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use diesel::{
    pg::PgConnection, result::Error, ExpressionMethods, Insertable, PgTextExpressionMethods,
    QueryDsl, Queryable, RunQueryDsl,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, str::FromStr};
use uuid::Uuid;
use validator::Validate;

use super::{
    glossary_history::{create_glossary_history, list_glossary_history},
    like::{list_likes, Like},
};
use crate::{
    response::{ApiError, ListResp, Message},
    schema::*,
    DBPool,
};

pub type Glossaries = ListResp<Glossary>;

#[derive(Debug, Clone, Deserialize, Serialize, Eq, PartialEq)]
pub struct Glossary {
    pub id: String,
    pub term: String,
    pub definition: String,
    pub revision: i32,
    pub likes: Vec<Like>,
    pub likes_count: i32,
    pub who: Option<String>,
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
            who: None,
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

    pub fn add_who(&self, who: String) -> Self {
        Self {
            who: Some(who),
            ..self.clone()
        }
    }
}

#[derive(Queryable, Insertable)]
#[diesel(table_name = glossary)]
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
            who: None,
            created_at: Utc.from_utc_datetime(&self.created_at),
            updated_at: Utc.from_utc_datetime(&self.updated_at),
        }
    }

    pub fn to_glossary_with_who(&self, who: Option<String>) -> Glossary {
        let mut glossary = self.to_glossary();
        glossary.who = who;

        glossary
    }

    pub fn to_glossary_with_who_from_db(&self, conn: &mut PgConnection) -> Glossary {
        let id = Uuid::from_str(&self.id.to_string()).unwrap();
        let histories = list_glossary_history(conn, id).unwrap_or_default();
        let who = histories.last().map(|h| h.who.clone()).unwrap_or_default();
        self.to_glossary_with_who(who)
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

fn list_glossary(conn: &mut PgConnection) -> Result<Vec<GlossaryDB>, Error> {
    use crate::schema::glossary::dsl::*;

    glossary.order(term.asc()).load(conn)
}

fn search_glossary(conn: &mut PgConnection, query: &str) -> Result<Vec<GlossaryDB>, Error> {
    use crate::schema::glossary::dsl::*;

    let search_pattern = format!("%{}%", query.to_lowercase());

    glossary
        .filter(
            term.ilike(&search_pattern)
                .or(definition.ilike(&search_pattern)),
        )
        .order(term.asc())
        .load(conn)
}

fn create_glossary(
    conn: &mut PgConnection,
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

fn get_glossary(conn: &mut PgConnection, _id: Uuid) -> Result<GlossaryDB, Error> {
    use crate::schema::glossary::dsl::*;

    glossary.filter(id.eq(_id)).first::<GlossaryDB>(conn)
}

fn update_glossary(
    conn: &mut PgConnection,
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

fn delete_glossary(conn: &mut PgConnection, _id: Uuid) -> Result<usize, Error> {
    use crate::schema::glossary::dsl::*;

    // Database CASCADE constraints handle deletion of dependent records
    // (glossary_history, likes) automatically
    diesel::delete(glossary.find(_id)).execute(conn)
}

fn list_popular_glossary(
    conn: &mut PgConnection,
    limit: Option<u8>,
) -> Result<Vec<Glossary>, Error> {
    use diesel::dsl;

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
        .filter(glossary::columns::id.eq_any(most_glossary_id_by_count))
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
pub async fn list(pool: web::Data<DBPool>) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    // Diesel does not support tokio (the asynchronous engine behind Actix),
    // so we have to run it in separate threads using the web::block
    let glossaries = web::block(move || list_glossary(&mut conn)).await??;

    let mut glossaries_by_alphabet: HashMap<String, Vec<Glossary>> = HashMap::new();
    let mut conn = pool.get().expect("could not get db connection from pool");

    glossaries.into_iter().for_each(|a| {
        let id = Uuid::from_str(&a.id.to_string()).unwrap();
        let likes = list_likes(&mut conn, id).unwrap_or_default();
        let histories = list_glossary_history(&mut conn, id).unwrap_or_default();
        let who = match histories.last() {
            Some(h) => h.who.clone().unwrap_or_default(),
            None => "".to_string(),
        };

        let character = a.term.chars().next().unwrap().to_uppercase();
        let b = glossaries_by_alphabet
            .entry(character.to_string())
            .or_insert_with(Vec::new);
        b.push(a.to_glossary().add_likes(likes).add_who(who));
    });

    Ok(web::Json(glossaries_by_alphabet as GroupedGlossary))
}

/// Create a new glossary
#[post("/glossary")]
pub async fn create(
    json: Json<GlossaryRequest>,
    req: HttpRequest,
    pool: web::Data<DBPool>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());
    let who_ = who.clone();

    let result = web::block(move || create_glossary(&mut conn, json, who)).await??;
    Ok(web::Json(result.to_glossary_with_who(who_)))
}

/// Find a glossary by id
#[get("/glossary/{id}")]
pub async fn get(
    pool: web::Data<DBPool>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");
    let mut conn2 = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    let glossary = web::block(move || get_glossary(&mut conn, glossary_id)).await??;
    Ok(web::Json(glossary.to_glossary_with_who_from_db(&mut conn2)))
}

/// Update a glossary by id
#[put("/glossary/{id}")]
pub async fn update(
    pool: web::Data<DBPool>,
    id: web::Path<String>,
    Json(value): Json<GlossaryRequest>,
    req: HttpRequest,
) -> actix_web::Result<impl Responder, ApiError> {
    let who = req
        .headers()
        .get(crate::AUTHENTICATED_USER_HEADER)
        .map(|email| email.to_str().unwrap().to_string());
    let who2 = who.clone();

    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    let glossary = web::block(move || {
        let mut conn = pool.get().expect("could not get db connection from pool");
        update_glossary(&mut conn, glossary_id, value.to_glossary().unwrap(), who)
    })
    .await??;

    Ok(web::Json(glossary.to_glossary_with_who(who2)))
}

/// Delete a glossary by id
#[delete("/glossary/{id}")]
pub async fn delete(
    pool: web::Data<DBPool>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");
    let glossary_id = Uuid::from_str(&id)
        .map_err(|_| ApiError::invalid_input("Invalid glossary ID format"))?;

    web::block(move || delete_glossary(&mut conn, glossary_id)).await??;
    Ok(web::Json(Message::new("deleted")))
}

#[derive(Deserialize)]
pub struct PopularQuery {
    pub limit: Option<u8>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

/// List popular glossaries
#[get("/glossary-popular")]
pub async fn list_popular(
    pool: web::Data<DBPool>,
    query: web::Query<PopularQuery>,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut conn = pool.get().expect("could not get db connection from pool");

    let glossaries = web::block(move || {
        let limit = query.limit;
        list_popular_glossary(&mut conn, limit)
    })
    .await??;

    Ok(web::Json(glossaries))
}

/// Search glossaries by term or definition
#[get("/glossary-search")]
pub async fn search(
    pool: web::Data<DBPool>,
    query: web::Query<SearchQuery>,
) -> actix_web::Result<impl Responder, ApiError> {
    let search_query = query.q.clone();

    if search_query.trim().is_empty() {
        return Err(ApiError::invalid_input("Search query cannot be empty"));
    }

    let mut conn = pool.get().expect("could not get db connection from pool");
    let results = web::block(move || search_glossary(&mut conn, &search_query)).await??;

    let mut conn2 = pool.get().expect("could not get db connection from pool");
    let glossaries: Vec<Glossary> = results
        .into_iter()
        .map(|g| g.to_glossary_with_who_from_db(&mut conn2))
        .collect();

    Ok(web::Json(Glossaries::from(&glossaries)))
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestContext;
    use crate::v1::like::create_like;
    use actix_web::{http::StatusCode, test, App};

    macro_rules! service_should_ok_and_return_json {
        ($app:expr, $req:expr) => {{
            let req = test::TestRequest::from($req).to_request();
            let resp = test::call_service(&$app, req).await;
            println!("{:?}", resp);

            assert!(resp.status().is_success());
            assert_eq!(
                resp.headers().get("content-type").unwrap(),
                "application/json"
            );

            resp
        }};
    }

    // Insert a glossary into database.
    // Than, using API to get list of glossaries
    #[actix_rt::test]
    async fn test_list_glossary() {
        let ctx = TestContext::new("test_list_glossary");
        let pool = web::Data::new(ctx.get_pool());
        let conn = &mut pool.get().expect("could not get db connection from pool");

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
            .execute(conn)
            .expect("could not insert glossary");
        diesel::insert_into(glossary::table)
            .values(item_2)
            .execute(conn)
            .expect("could not insert glossary");

        let app = test::init_service(App::new().app_data(pool).service(list)).await;

        // Response should be OK and application/json
        let req = test::TestRequest::get().uri("/glossary");
        let resp = service_should_ok_and_return_json!(app, req);

        // The response should be:
        // {
        //   "T": [
        //      { "term": "test_term_1", "definition": "test_definition_1", ... },
        //      { "term": "test_term_2", "definition": "test_definition_2", ... },
        //   ]
        // }
        let response: GroupedGlossary = test::read_body_json(resp).await;
        assert_eq!(response.keys().len(), 1);
        let values = response.get("T").unwrap();
        assert_eq!(values.len(), 2);
    }

    // Direct into data to Database.
    // Than, using API to get glossary
    #[actix_rt::test]
    async fn test_get_glossary() {
        let ctx = TestContext::new("test_get_glossary");
        let pool = web::Data::new(ctx.get_pool());
        let conn = &mut pool.get().expect("could not get db connection from pool");

        let glossary_id = Uuid::new_v4();
        let api_url = format!("/glossary/{}", glossary_id);

        let item = GlossaryDB {
            id: glossary_id,
            term: "test_term_1".to_string(),
            revision: 1,
            definition: "test_definition_1".to_string(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        };

        // Insert two glossaries
        diesel::insert_into(glossary::table)
            .values(item)
            .execute(conn)
            .expect("could not insert glossary");

        let app = test::init_service(App::new().app_data(pool).service(get)).await;

        // Response should be OK and application/json
        let req = test::TestRequest::get().uri(&api_url);
        let resp = service_should_ok_and_return_json!(app, req);

        // Body JSON should contains the term and definition
        let resp: Glossary = test::read_body_json(resp).await;
        assert_eq!(resp.term, "test_term_1");
        assert_eq!(resp.definition, "test_definition_1");
        assert_eq!(resp.revision, 1);
    }

    // Get glossary that does not exist - should return 404 NOT FOUND
    #[actix_rt::test]
    async fn test_get_glossary_not_exists() {
        let ctx = TestContext::new("test_get_glossary_not_exists");
        let pool = web::Data::new(ctx.get_pool());

        let non_exists_id = Uuid::new_v4();
        let api_url = format!("/glossary/{}", non_exists_id);

        // Init app
        let app = test::init_service(App::new().app_data(pool).service(get)).await;

        // Response should be not found
        let req = test::TestRequest::get().uri(&api_url).to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // Get glossary with invalid id format - should return 400 BAD REQUEST
    #[actix_rt::test]
    async fn test_get_glossary_invalid_id() {
        let ctx = TestContext::new("test_get_glossary_invalid_id");
        let pool = web::Data::new(ctx.get_pool());

        let invalid_glossary_id = "abc1234";
        let api_url = format!("/glossary/{}", invalid_glossary_id);

        // Init app
        let app = test::init_service(App::new().app_data(pool).service(get)).await;

        // Response should be bad request for invalid ID format
        let req = test::TestRequest::get().uri(&api_url).to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // Using API to create glossary
    #[actix_rt::test]
    async fn test_create_glossary() {
        let ctx = TestContext::new("test_create_glossary");
        let pool = web::Data::new(ctx.get_pool());

        let app = test::init_service(App::new().app_data(pool).service(create)).await;
        let glossary_req = GlossaryRequest {
            term: Some("test_term_1".to_string()),
            definition: Some("test_definition_1".to_string()),
        };

        // Response should be OK and application/json
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&glossary_req);
        let resp = service_should_ok_and_return_json!(app, req);

        // Body JSON should contains the term and definition
        let resp: Glossary = test::read_body_json(resp).await;
        assert_eq!(resp.term, "test_term_1");
        assert_eq!(resp.definition, "test_definition_1");
    }

    // Using API to create glossary with invalid JSON
    #[actix_rt::test]
    async fn test_create_glossary_invalid_json() {
        let ctx = TestContext::new("test_create_glossary_invalid_json");
        let pool = web::Data::new(ctx.get_pool());

        let app = test::init_service(App::new().app_data(pool).service(create)).await;
        let glossary_req = b"{\"invalid\": \"json\"}";

        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&glossary_req)
            .to_request();
        let resp = test::call_service(&app, req).await;

        // Response should be bad request
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // Using API to create glossary. Than, using API to get glossary.
    #[actix_rt::test]
    async fn test_create_glossary_then_get() {
        let ctx = TestContext::new("test_create_glossary_then_get");
        let pool = web::Data::new(ctx.get_pool());

        let app = test::init_service(App::new().app_data(pool).service(create).service(get)).await;
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            });

        // Response should be OK and application/json
        let resp = service_should_ok_and_return_json!(app, req);

        // Get glossary from api
        let response_of_create: Glossary = test::read_body_json(resp).await;

        // Response should be OK and application/json
        let req = test::TestRequest::get().uri(&format!("/glossary/{}", response_of_create.id));
        let resp = service_should_ok_and_return_json!(app, req);
        let response_of_get: Glossary = test::read_body_json(resp).await;

        // Response of API create should be equal to response of API get
        assert_eq!(response_of_create, response_of_get);
    }

    // Using API to create glossary. Than, using API to update glossary.
    #[actix_rt::test]
    async fn test_create_glossary_then_update() {
        let ctx = TestContext::new("test_create_glossary_then_update");
        let pool = web::Data::new(ctx.get_pool());

        let services = App::new().app_data(pool).service(create).service(update);
        let app = test::init_service(services).await;

        // Response should be OK and application/json
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            });
        let resp = service_should_ok_and_return_json!(app, req);

        let response_of_create: Glossary = test::read_body_json(resp).await;

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
            });
        let resp = service_should_ok_and_return_json!(app, req);

        // Body JSON should contains the term and definition
        let response_of_update: Glossary = test::read_body_json(resp).await;
        assert_eq!(response_of_update.term, "test_term_1_updated");
        assert_eq!(response_of_update.definition, "test_definition_1_updated");
        assert_eq!(response_of_update.revision, 1);
    }

    // Using API to create glossary. Than, using API to delete glossary.
    #[actix_rt::test]
    async fn test_create_glossary_then_delete() {
        let ctx = TestContext::new("test_create_glossary_then_delete");
        let pool = web::Data::new(ctx.get_pool());

        let app = test::init_service(
            App::new()
                .app_data(pool)
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
            });

        // Response should be OK and application/json
        let resp = service_should_ok_and_return_json!(app, req);

        // Response of create
        let response_of_create: Glossary = test::read_body_json(resp).await;

        // Verify that insert correctly
        assert_eq!(response_of_create.term, "test_term_1");
        assert_eq!(response_of_create.definition, "test_definition_1");
        assert_eq!(response_of_create.revision, 0);

        // Delete glossary should success
        let req = test::TestRequest::delete().uri(&format!("/glossary/{}", response_of_create.id));
        let resp = service_should_ok_and_return_json!(app, req);

        // Response should be: {"message": "deleted"}
        let response_of_delete: Message = test::read_body_json(resp).await;
        assert_eq!(response_of_delete.message, "deleted");

        // For sure, we get the deleted glossary via API again
        let req = test::TestRequest::get().uri(&format!("/glossary/{}", response_of_create.id));
        let resp = test::call_service(&app, req.to_request()).await;
        println!("{:?}", resp.status());
        // Response should be NOT_FOUND
        assert!(resp.status().is_client_error());
    }

    // Fast test list popular glossaries
    // By default, the list should be empty
    #[actix_rt::test]
    async fn test_list_popular_glossaries_empty() {
        let ctx = TestContext::new("test_list_popular_glossaries");
        let pool = web::Data::new(ctx.get_pool());

        let app = test::init_service(App::new().app_data(pool).service(list_popular)).await;

        // Response should be OK and application/json
        let req = test::TestRequest::get().uri("/glossary-popular");
        let resp = service_should_ok_and_return_json!(app, req);

        // Response
        let response_of_list_popular: Vec<Glossary> = test::read_body_json(resp).await;

        // By default: the list popular should be empty if there is not likes on any glossary
        assert!(response_of_list_popular.is_empty());
    }

    // Test list popular glossaries with inserted glossaries
    #[actix_rt::test]
    async fn test_list_popular_glossaries_with_inserted_glossaries() {
        let ctx = TestContext::new("test_list_popular_glossaries_with_inserted_glossaries");
        let pool = web::Data::new(ctx.get_pool());
        let mut conn = pool.get().expect("could not get connection from pool");

        let app = test::init_service(
            App::new()
                .app_data(pool)
                .service(create)
                .service(list_popular),
        )
        .await;

        // Response should be OK and application/json
        let req = test::TestRequest::post()
            .uri("/glossary")
            .set_json(&GlossaryRequest {
                term: Some("test_term_1".to_string()),
                definition: Some("test_definition_1".to_string()),
            });
        let resp = service_should_ok_and_return_json!(app, req);

        // Verify that insert correctly
        let response_of_create: Glossary = test::read_body_json(resp).await;
        assert_eq!(response_of_create.term, "test_term_1");
        assert_eq!(response_of_create.definition, "test_definition_1");
        assert_eq!(response_of_create.revision, 0);

        let glossary_id = Uuid::from_str(&response_of_create.id).unwrap();
        let _ = create_like(&mut conn, glossary_id, None);

        // Get the list popular
        let req = test::TestRequest::get().uri("/glossary-popular");
        let resp = service_should_ok_and_return_json!(app, req);
        let response_of_list_popular: Vec<Glossary> = test::read_body_json(resp).await;
        assert_eq!(response_of_list_popular.len(), 1);
    }
}
