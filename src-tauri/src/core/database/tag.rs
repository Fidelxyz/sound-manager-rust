use std::collections::{HashMap, HashSet};

use serde::Serialize;

use super::ROOT_TAG_ID;

pub type TagId = i32;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    pub color: i32,

    #[serde(skip)]
    pub parent_id: TagId,
    #[serde(skip)]
    pub position: i32,
    #[serde(skip)]
    pub children: HashSet<TagId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagNode<'a> {
    pub tag: &'a Tag,
    pub children: Vec<TagNode<'a>>,
}

impl TagNode<'_> {
    fn new(id: TagId, tags: &HashMap<TagId, Tag>) -> TagNode {
        let tag = tags.get(&id).unwrap();

        let mut children = tag
            .children
            .iter()
            .map(|child_id| TagNode::new(*child_id, tags))
            .collect::<Vec<_>>();

        children.sort_by_key(|child| child.tag.position);

        TagNode { tag, children }
    }

    pub fn build(tags: &HashMap<TagId, Tag>) -> Vec<TagNode> {
        let root = TagNode::new(ROOT_TAG_ID, tags);
        root.children
    }
}
