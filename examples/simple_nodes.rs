use internment::ArcIntern;
use once_cell::sync::Lazy;
use osmium::{graph::{concrete::{Graph, OUTPUT}, NODES}, exec::run};
use serde_json::json;

fn main() {
    let mut graph = Graph::new();

    let value = graph.insert("Value", vec![json! { 0.123 }], vec![]);

    graph.connect(value, 0, ArcIntern::<String>::from_ref(OUTPUT), 0);

    let final_graph = graph.finalize(&Lazy::force(&NODES)).unwrap();

    let results = run(final_graph, osmium::exec::GraphContext { dimensions: (2, 2) }).unwrap();

    println!("{results:#?}")
}