use serde_json::Value;

pub fn passthrough(meta: &[Value], inputs: &[Value]) -> Vec<Value> {
    meta.iter().chain(inputs.iter()).cloned().collect()
}
