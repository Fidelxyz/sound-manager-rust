use std::collections::HashSet;

use serde::Serialize;

pub type TagId = i32;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: i32,
    #[serde(skip)]
    pub parent_id: TagId,
    pub position: i32,
    pub children: HashSet<TagId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagNode<'a> {
    pub tag: &'a Tag,
    pub children: Vec<TagNode<'a>>,
}
