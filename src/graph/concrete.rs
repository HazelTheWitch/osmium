use std::{collections::HashMap, ops::Deref};

use rand::{SeedableRng, rngs::SmallRng, distributions::{Alphanumeric, DistString}};
use serde::{Serialize, Deserialize};
use serde_json::{Value, from_value, json};
use thiserror::Error;
use internment::ArcIntern;
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};

use crate::exec::GraphContext;

use super::types::{NodeType, DataType};

pub static OUTPUT: &str = "_output";
pub static INPUT: &str = "_input";

#[derive(Debug, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: HashMap<ArcIntern<String>, Node>,
    pub connections: Vec<Connection>,
    #[serde(skip, default = "SmallRng::from_entropy")]
    rng: SmallRng,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Node {
    pub node_type: ArcIntern<String>,
    pub meta: Vec<Value>,
    pub inputs: Vec<SlotValue>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SlotValue {
    Connected,
    Value(Value),
    None,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Connection {
    pub input: NodeAddress,
    pub output: NodeAddress,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct NodeAddress {
    pub node: ArcIntern<String>,
    pub slot: usize,
}

#[derive(Debug)]
pub struct FinalizedGraph<'nodes> {
    pub(crate) graph: Graph,
    pub(crate) node_types: &'nodes HashMap<ArcIntern<String>, NodeType>,
}

#[derive(Debug)]
pub enum FieldType {
    Meta,
    Input,
    Output,
}

#[derive(Debug)]
pub enum SlotType {
    Input,
    Output,
}

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("cyclical dependency found")]
    Cyclic,
    #[error("invalid node type `{0}`")]
    InvalidNode(String),
    #[error("missing slot value for `{0:?}`")]
    MissingValue(FieldType),
    #[error("invalid slot index")]
    InvalidSlots,
}

impl <'nodes> Deref for FinalizedGraph<'nodes> {
    type Target = Graph;

    fn deref(&self) -> &Self::Target {
        &self.graph
    }
}

impl Default for Graph {
    fn default() -> Self {
        Self {
            rng: SmallRng::from_entropy(),
            nodes: Default::default(),
            connections: Default::default(),
        }
    }
}

impl Graph {
    pub fn new() -> Self {
        let mut graph: Graph = Default::default();

        graph.nodes.insert(ArcIntern::<String>::from_ref(INPUT), Node { node_type: ArcIntern::<String>::from_ref("Input"), meta: vec![], inputs: vec![] });
        graph.nodes.insert(ArcIntern::<String>::from_ref(OUTPUT), Node { node_type: ArcIntern::<String>::from_ref("Output"), meta: vec![], inputs: vec![SlotValue::None] });

        graph
    }

    pub fn insert(&mut self, node: impl Into<String>, meta: Vec<Value>, inputs: Vec<SlotValue>) -> ArcIntern<String> {
        let mut key;

        loop {
            key = ArcIntern::new(Alphanumeric.sample_string(&mut self.rng, 16));

            if !self.nodes.contains_key(&key) {
                break;
            }
        }

        let node = Node {
            node_type: ArcIntern::new(node.into()),
            meta,
            inputs,
        };

        self.nodes.insert(key.clone(), node);

        key
    }

    pub fn connect(&mut self, output_node: ArcIntern<String>, out_slot: usize, input_node: ArcIntern<String>, input_slot: usize) -> bool {
        let output = NodeAddress {
            node: output_node.clone(),
            slot: out_slot,
        };

        let input = NodeAddress {
            node: input_node.clone(),
            slot: input_slot,
        };

        let connection = Connection {
            input,
            output,
        };

        if self.connections.contains(&connection) || !self.nodes.contains_key(&output_node) || !self.nodes.contains_key(&input_node) {
            return false;
        }

        let Some(in_node) = self.nodes.get_mut(&input_node) else {
            return false;
        };

        let Some(in_slot) = in_node.inputs.get_mut(input_slot) else {
            return false;
        };

        *in_slot = SlotValue::Connected;

        self.connections.push(connection);

        true
    }

    pub fn slot_connected_from(&self, node: ArcIntern<String>, slot: usize) -> Option<&Connection> {
        let input = NodeAddress {
            node,
            slot,
        };

        self.connections
            .iter()
            .find(|c| c.input == input)
    }

    pub fn finalize(self, nodes: &HashMap<ArcIntern<String>, NodeType>) -> Result<FinalizedGraph, GraphError> {
        let mut dependency_graph = petgraph::Graph::<ArcIntern<String>, ()>::new();
        let mut addresses = HashMap::new();

        for (node_id, node) in self.nodes.iter() {
            let Some(node_type) = nodes.get(&node.node_type) else {
                return Err(GraphError::InvalidNode(node.node_type.to_string()));
            };

            if node.meta.len() != node_type.meta.len() {
                return Err(GraphError::MissingValue(FieldType::Meta));
            }

            if node.inputs.len() != node_type.inputs.len() {
                return Err(GraphError::MissingValue(FieldType::Input));
            }

            addresses.insert(node_id.clone(), dependency_graph.add_node(node_id.clone()));
        }

        for connection in self.connections.iter() {
            let input = self.nodes.get(&connection.input.node).unwrap();
            let Some(input_type) = nodes.get(&input.node_type) else {
                return Err(GraphError::InvalidNode(input.node_type.to_string()));
            };

            let output = self.nodes.get(&connection.output.node).unwrap();
            let Some(output_type) = nodes.get(&output.node_type) else {
                return Err(GraphError::InvalidNode(output.node_type.to_string()));
            };

            let Some(input_slot) = input_type.inputs.get(connection.input.slot) else {
                return Err(GraphError::InvalidSlots);
            };

            let Some(output_slot) = output_type.outputs.get(connection.output.slot) else {
                return Err(GraphError::InvalidSlots);
            };

            if !DataType::broadcasts(&output_slot.data_type, &input_slot.data_type) {
                return Err(GraphError::InvalidSlots);
            }

            dependency_graph.add_edge(*addresses.get(&connection.input.node).unwrap(), *addresses.get(&connection.output.node).unwrap(), ());
        }

        if petgraph::algo::is_cyclic_directed(&dependency_graph) {
            return Err(GraphError::Cyclic);
        }

        Ok(FinalizedGraph {
            graph: self,
            node_types: nodes,
        })
    }
}

impl<'nodes> FinalizedGraph<'nodes> {
    pub fn slot_datatype(&self, address: &NodeAddress, slot_type: SlotType) -> Option<DataType> {
        let node_type_key = &self.nodes.get(&address.node)?.node_type;

        let node_type = self.node_types.get(node_type_key)?;

        Some(match slot_type {
            SlotType::Input => node_type.inputs.get(address.slot)?.data_type,
            SlotType::Output => node_type.outputs.get(address.slot)?.data_type,
        })
    }
}

#[derive(Debug, Error)]
pub enum BroadcastingError {
    #[error("can not broadcast {0:?} to {1:?}")]
    CanNotBroadcast(DataType, DataType),
    #[error("can not turn value into concrete data type")]
    InvalidValue(#[from] serde_json::Error),
}

pub fn broadcast(value: Value, from: &DataType, to: &DataType, ctx: &GraphContext) -> Result<Value, BroadcastingError> {
    if !DataType::broadcasts(from, to) {
        return Err(BroadcastingError::CanNotBroadcast(*from, *to));
    }

    if from == to {
        return Ok(value);
    }

    let image_size = ctx.dimensions.0 * ctx.dimensions.1;

    Ok(match (from, to) {
        (DataType::Scalar, _) => {
            let value: f64 = from_value(value)?;

            let as_u8 = (255.0 * value) as u8;

            match to {
                DataType::Vec2 => json! { [value, value] },
                DataType::Vec3 => json! { [value, value, value] },
                DataType::Color => json! { [as_u8, as_u8, as_u8, 255] },
                DataType::Texture => {
                    let data = STANDARD_NO_PAD.encode([as_u8, as_u8, as_u8, 255].repeat(image_size));

                    json! { data }
                },
                _ => unreachable!()
            }
        },
        (DataType::Vec3, DataType::Color | DataType::Texture) => {
            let color_f64: (f64, f64, f64) = from_value(value)?;

            let color = [(255.0 * color_f64.0) as u8, (255.0 * color_f64.1) as u8, (255.0 * color_f64.2) as u8, 255];

            match to {
                DataType::Color => json! { color },
                DataType::Texture => {
                    let data = STANDARD_NO_PAD.encode(color.repeat(image_size));

                    json! { data }
                },
                _ => unreachable!(),
            }
        },
        (DataType::Color, DataType::Vec3 | DataType::Texture) => {
            let color: [u8; 4] = from_value(value)?;

            match to {
                DataType::Vec3 => {
                    let vec3: Vec<f64> = color.iter().map(|c| (*c as f64) / 255.0).take(3).collect();

                    json! { vec3 }
                },
                DataType::Texture => {
                    let data = STANDARD_NO_PAD.encode(color.repeat(image_size));

                    json! { data }
                },
                _ => unreachable!(),
            }
        },
        _ => unreachable!(),
    })
}
