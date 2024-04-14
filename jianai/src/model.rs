use diesel::prelude::*;
use serde_json::Value;
use crate::types::*;
use serde::Serialize;

#[derive(Clone, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::images)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Image {
    pub id: i32,
    pub filename: String,
    pub digest: Vec<u8>,
    pub metadata: Option<Value>,
    pub segmented: bool,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::segments)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Segment {
    pub id: i32,
    pub image_id: i32,
    pub bounding_box: Box,
    pub identified_as: Option<i32>,
    pub tagged_as: Option<i32>,
    pub low_quality: bool,
}

#[derive(Debug, Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::schema::tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Tag {
    pub id: i32,
    pub tag: String,
}

#[derive(Serialize)]
pub struct SegmentWithTag {
    pub id: i32,
    pub image_id: i32,
    pub bounding_box: Box,
    pub identified_as: Option<Tag>,
    pub tagged_as: Option<Tag>,
    pub low_quality: bool,
}
