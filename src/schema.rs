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
    glossary_history (id) {
        id -> Uuid,
        term -> Varchar,
        definition -> Text,
        revision -> Int4,
        who -> Nullable<Varchar>,
        created_at -> Timestamp,
        glossary_id -> Uuid,
    }
}

table! {
    likes (id) {
        id -> Uuid,
        created_at -> Timestamp,
        glossary_id -> Uuid,
        who -> Nullable<Varchar>,
    }
}

joinable!(glossary_history -> glossary (glossary_id));
joinable!(likes -> glossary (glossary_id));

allow_tables_to_appear_in_same_query!(glossary, glossary_history, likes,);
