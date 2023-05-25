use internment::ArcIntern;
use once_cell::sync::Lazy;
use osmium::{graph::{concrete::{Graph, SlotValue}, NODES}, exec::run};
use serde_json::json;

fn main() {
    let mut graph = Graph::new();

    let value = graph.insert("Value", vec![json! { 0.123 }], vec![]);
    let save = graph.insert("Save", vec![json! { "./test.png" }], vec![SlotValue::None]);

    graph.connect(value, 0, save, 0);

    let final_graph = graph.finalize(&Lazy::force(&NODES)).unwrap();

    let results = run(final_graph, osmium::exec::GraphContext { dimensions: (64, 64) }).unwrap();

    println!("{results:#?}")
}