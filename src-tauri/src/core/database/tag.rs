use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct Tag {
    pub id: i32,
    pub parent_id: i32,
    pub position: i32,
    pub children_ids: HashSet<i32>,
    pub name: String,
    pub color: i32,
}

#[derive(Debug)]
pub struct TagNode<'a> {
    pub tag: &'a Tag,
    pub children: Vec<TagNode<'a>>,
}

impl TagNode<'_> {
    fn new(id: i32, tags: &HashMap<i32, Tag>) -> TagNode {
        let tag = tags.get(&id).unwrap();

        let mut children = tag
            .children_ids
            .iter()
            .map(|child_id| TagNode::new(*child_id, tags))
            .collect::<Vec<_>>();

        children.sort_by_key(|child| child.tag.position);

        TagNode { tag, children }
    }

    pub fn build<'a>(
        tags: &HashMap<i32, Tag>,
        root_tag_ids: impl IntoIterator<Item = &'a i32>,
    ) -> Vec<TagNode> {
        let mut root_tags = root_tag_ids
            .into_iter()
            .map(|root_tag_id| TagNode::new(*root_tag_id, tags))
            .collect::<Vec<_>>();

        root_tags.sort_by_key(|root_tag| root_tag.tag.position);

        root_tags
    }
}
