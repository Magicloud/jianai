// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "box", schema = "pg_catalog"))]
    pub struct Box;
}

diesel::table! {
    images (id) {
        id -> Int4,
        filename -> Text,
        digest -> Bytea,
        metadata -> Nullable<Jsonb>,
        segmented -> Bool,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::Box;

    segments (id) {
        id -> Int4,
        image_id -> Int4,
        bounding_box -> Box,
        identified_as -> Nullable<Int4>,
        tagged_as -> Nullable<Int4>,
        low_quality -> Bool,
    }
}

diesel::table! {
    tags (id) {
        id -> Int4,
        tag -> Text,
    }
}

diesel::joinable!(segments -> images (image_id));

diesel::allow_tables_to_appear_in_same_query!(
    images,
    segments,
    tags,
);
