pub mod concrete;
pub mod types;

use std::collections::HashMap;

use internment::ArcIntern;
use once_cell::sync::Lazy;

use self::types::NodeType;

pub static NODES: Lazy<HashMap<ArcIntern<String>, NodeType>> =
    Lazy::new(|| serde_yaml::from_str(include_str!("../nodes.yaml")).expect("invalid nodes.yaml"));

pub(super) fn get_nodes() -> &'static HashMap<ArcIntern<String>, NodeType> {
    Lazy::force(&NODES)
}
