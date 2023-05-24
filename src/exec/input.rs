use serde_json::{Value, json};

use super::GraphContext;

pub fn node_info(ctx: &GraphContext) -> Vec<Value> {
    vec![json! { [ctx.dimensions.0, ctx.dimensions.1] }]
}