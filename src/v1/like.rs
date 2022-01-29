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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Like {
    pub id: String,
    pub created_at: DateTime<Utc>,
}

impl Default for Like {
    fn default() -> Self {
        Self::new()
    }
}

impl Like {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: Utc::now(),
        }
    }

    pub fn to_like_db(&self, glossary_id: Uuid) -> LikeDB {
        LikeDB {
            id: Uuid::from_str(self.id.as_str()).unwrap(),
            created_at: self.created_at.naive_utc(),
            glossary_id,
        }
    }
}

#[derive(Queryable, Insertable)]
#[table_name = "likes"]
pub struct LikeDB {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub glossary_id: Uuid,
}

impl LikeDB {
    pub fn to_like(&self) -> Like {
        Like {
            id: self.id.to_string(),
            created_at: DateTime::<Utc>::from_utc(self.created_at, Utc),
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

pub fn create_like(conn: &DBPooledConnection, _glossary_id: Uuid) -> Result<Like, ErrorResp> {
    use crate::schema::likes::dsl::*;

    let like = Like::new();

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
) -> Result<HttpResponse, ErrorResp> {
    let conn = pool.get().expect("could not get db connection from pool");

    let glossary_id = Uuid::from_str(&path.0).expect("could not parse glossary id");
    let like = web::block(move || create_like(&conn, glossary_id)).await;

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
