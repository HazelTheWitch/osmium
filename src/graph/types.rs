use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeType {
    pub name: String,
    pub meta: Vec<MetaInfo>,
    pub inputs: Vec<InputInfo>,
    pub outputs: Vec<OutputInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub meta_type: MetaType,
    pub default_value: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MetaType {
    Simple(DataType),
    FilePath,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InputInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: DataType,
    pub default_value: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OutputInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: DataType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub enum DataType {
    Scalar,
    Vec2,
    Vec3,
    Color,
    Texture,
}

impl DataType {
    pub fn broadcasts(from: &Self, to: &Self) -> bool {
        match (from, to) {
            (a, b) if a == b => true,
            (Self::Scalar, _) => true,
            (Self::Vec3, Self::Color | Self::Texture) => true,
            (Self::Color, Self::Vec3 | Self::Texture) => true,
            _ => false,
        }
    }
}