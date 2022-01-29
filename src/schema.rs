table! {
    glossary (id) {
        id -> Uuid,
        term -> Varchar,
        definition -> Text,
        revision -> Int4,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    likes (id) {
        id -> Uuid,
        created_at -> Timestamp,
        glossary_id -> Uuid,
    }
}

joinable!(likes -> glossary (glossary_id));

allow_tables_to_appear_in_same_query!(glossary, likes,);
