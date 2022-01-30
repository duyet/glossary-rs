use chrono::{NaiveDateTime, Utc};
use diesel::RunQueryDsl;
use diesel::{Insertable, Queryable};
use log::info;
use uuid::Uuid;

use crate::schema::*;
use crate::DBPooledConnection;

#[derive(Debug, Queryable, Insertable)]
#[table_name = "glossary_history"]
pub struct GlossaryHistoryDB {
    pub id: Uuid,
    pub term: String,
    pub definition: String,
    pub who: Option<String>,
    pub revision: i32,
    pub created_at: NaiveDateTime,
    pub glossary_id: Uuid,
}

pub fn create_glossary_history(
    conn: &DBPooledConnection,
    term: String,
    definition: String,
    who: Option<String>,
    revision: i32,
    glossary_id: Uuid,
) {
    let _glossary_history = GlossaryHistoryDB {
        id: Uuid::new_v4(),
        term,
        definition,
        revision,
        glossary_id,
        who,
        created_at: Utc::now().naive_utc(),
    };

    info!("Insert a history revison: {:?}", _glossary_history);
    let _ = diesel::insert_into(glossary_history::table)
        .values(_glossary_history)
        .execute(conn);
}
