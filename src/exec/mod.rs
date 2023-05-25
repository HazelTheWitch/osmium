use std::collections::HashMap;

use internment::ArcIntern;
use serde_json::Value;
use thiserror::Error;

use crate::graph::concrete::{FinalizedGraph, SlotValue, broadcast, SlotType, BroadcastingError};

use self::{passthrough::passthrough, input::node_info, save::save_texture};

mod passthrough;
mod input;
mod save;

pub struct GraphContext {
    pub dimensions: (usize, usize),
}

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("unknown node type `{0}`")]
    UnknownNode(ArcIntern<String>),
    #[error("disconnected slot `{0}`")]
    DisconnectedSlot(usize),
    #[error("node did not produce enough values")]
    ValueError,
    #[error("broadcasting error")]
    BroadcastingError(#[from] BroadcastingError),
    #[error("error (de)serializing values")]
    JSONError(#[from] serde_json::Error),
    #[error("error processing image")]
    ImageError(#[from] image::ImageError),
}

pub fn execute(node_type: ArcIntern<String>, meta: &[Value], inputs: &[Value], ctx: &GraphContext) -> Result<Vec<Value>, ExecutionError> {
    match node_type.as_str() {
        "Value" => Ok(passthrough(meta, inputs)),
        "Save" => save_texture(meta, inputs, ctx),
        "Input" => Ok(node_info(ctx)),
        _ => Err(ExecutionError::UnknownNode(node_type)),
    }
}

fn solve_for(node_id: ArcIntern<String>, graph: &FinalizedGraph, ctx: &GraphContext, cache: &mut HashMap<ArcIntern<String>, Vec<Value>>) -> Result<Vec<Value>, ExecutionError> {
    if let Some(outputs) = cache.get(&node_id) {
        return Ok(outputs.clone());
    }

    let Some(node) = graph.nodes.get(&node_id) else {
        return Err(ExecutionError::UnknownNode(node_id));
    };

    let mut inputs = Vec::<Value>::with_capacity(node.inputs.len());

    for (index, input) in node.inputs.iter().enumerate() {
        match input {
            SlotValue::Connected => {
                let Some(connection) = graph.slot_connected_from(node_id.clone(), index) else {
                    return Err(ExecutionError::DisconnectedSlot(index));
                };

                let Some(value) = solve_for(connection.output.node.clone(), graph, ctx, cache)?.get(connection.output.slot).cloned() else {
                    return Err(ExecutionError::ValueError);
                };

                let Some(output_type) = graph.slot_datatype(&connection.output, SlotType::Output) else {
                    return Err(ExecutionError::ValueError);
                };

                let Some(input_type) = graph.slot_datatype(&connection.input, SlotType::Input) else {
                    return Err(ExecutionError::ValueError);
                };

                inputs.push(broadcast(value, &output_type, &input_type, ctx)?);
            },
            SlotValue::Value(v) => inputs.push(v.clone()),
            SlotValue::None => return Err(ExecutionError::DisconnectedSlot(index)),
        }
    }

    let outputs = execute(node.node_type.clone(), &node.meta, &inputs, ctx)?;

    cache.insert(node_id, outputs.clone());

    Ok(outputs)
}

pub fn run(graph: FinalizedGraph, ctx: GraphContext) -> Result<HashMap<ArcIntern<String>, Vec<Value>>, ExecutionError> {
    let mut cache: HashMap<ArcIntern<String>, Vec<Value>> = HashMap::new();

    for (node, _) in graph.nodes.iter() {
        solve_for(node.clone(), &graph, &ctx, &mut cache)?;
    }

    Ok(cache)
}

#[cfg(test)]
mod tests {
    use internment::ArcIntern;
    use serde_json::json;

    use super::execute;

    #[test]
    fn passthrough() {
        let meta = vec![json! { 12 }];
        let inputs = vec![json! { 24 }];
        let result = execute(ArcIntern::<String>::from_ref("Output"), &meta, &inputs, &super::GraphContext { dimensions: (16, 16) }).unwrap();

        assert_eq!(result, vec![json! { 12 }, json! { 24 }]);
    }

    #[test]
    fn node_info() {
        let graph_info = super::GraphContext { dimensions: (16, 16) };
        let meta = vec![];
        let inputs = vec![];
        let result = execute(ArcIntern::<String>::from_ref("Input"), &meta, &inputs, &graph_info).unwrap();

        assert_eq!(result, vec![json! { [16, 16] }]);
    }
}