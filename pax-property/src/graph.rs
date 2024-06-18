use core::panic;
use std::collections::{HashMap, HashSet, VecDeque};

use crate::{property_table::{PropertyTable, PropertyType}, PropertyId};
use petgraph::{algo::toposort, dot::{Config, Dot}, visit::EdgeRef};
use petgraph::graph::{DiGraph, NodeIndex};
use svg::node::element::{Circle, Definitions, Group, Line, Marker, Polygon, Text};
use svg::Document;
use std::fs::File;
use std::io::Write;
use std::path::Path;


// pub struct PropertyData {
//     // Data associated with the property
//     pub value: Box<dyn Any>,
//     // Closures to run when this property is set
//     pub subscriptions: Subscriptions,
//     // The type of the property
//     pub property_type: PropertyType,
//     // List of properties that this property depends on
//     pub inbound: HashSet<PropertyId>,
//     // List of properties that depend on this value
//     pub outbound: HashSet<PropertyId>,
//     // Topologically sorted dependencies (None if not computed yet)
//     pub dependents_to_update: Option<Vec<PropertyId>>,
//     // Type agnostic transition manager
//     pub transition_manager: Option<TransitionManagerWrapper>,
// }


impl PropertyTable {

    // Function that recurses up inbound properties and deletes saved dependents to update so it's recommputed
    pub fn clear_memoized_dependents(&self, id: PropertyId) {
        let inbound = {
            let mut sm = self.properties.borrow_mut();
            let entry = sm.get_mut(&id).expect("Property not found");
            entry.data.dependents_to_update = None;
            entry.data.inbound.clone()
        };
        for &inbound_id in inbound.iter() {
            self.clear_memoized_dependents(inbound_id);
        }
    }



    // Function to perform a topological sort on affected properties and return a sorted vector of property ids
    pub fn topological_sort_affected(&self, start_id: PropertyId) -> Vec<PropertyId> {
        let mut in_degree = HashMap::new();
        let mut sorted = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        // Initialize the in-degree of affected properties
        fn initialize_in_degree(
            table: &PropertyTable,
            id: PropertyId,
            in_degree: &mut HashMap<PropertyId, usize>,
            visited: &mut HashSet<PropertyId>,
        ) {
            if !visited.insert(id) {
                return;
            }

            let sm = table.properties.borrow();
            let entry = sm.get(&id).expect("Property not found");

            in_degree.entry(id).or_insert(0);
            for &out_id in entry.data.outbound.iter() {
                *in_degree.entry(out_id).or_insert(0) += 1;
                initialize_in_degree(table, out_id, in_degree, visited);
            }
        }

        // Start with the given property ID and initialize in-degree for affected properties
        initialize_in_degree(self, start_id, &mut in_degree, &mut visited);

        // Add properties with in-degree 0 to the queue
        for (&id, &degree) in in_degree.iter() {
            if degree == 0 {
                queue.push_back(id);
            }
        }

        while let Some(id) = queue.pop_front() {
            sorted.push(id);
            let outbound = {
                let sm = self.properties.borrow();
                sm.get(&id).expect("Property not found").data.outbound.clone()
            };
            for &out_id in outbound.iter() {
                let degree = in_degree.get_mut(&out_id).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(out_id);
                }
            }
        }

        sorted[1..].to_vec()

    }

    pub fn render_graph_to_file(&self, file_path: &str) {
        let sm = self.properties.borrow();
        let mut graph = DiGraph::new();
        let mut indices = HashMap::new();

        // Add nodes
        for (id, entry) in sm.iter() {
            let node = graph.add_node(id.to_string());
            indices.insert(id, node);
        }

        // Add edges
        for (id, entry) in sm.iter() {
            let source = *indices.get(id).unwrap();
            for dep_id in &entry.data.inbound {
                let target = *indices.get(dep_id).unwrap();
                graph.add_edge(target, source, ()); // Note the direction for dependency
            }
        }

        // Perform topological sort to organize the graph as a DAG
        let topo_sorted_nodes = match toposort(&graph, None) {
            Ok(nodes) => nodes,
            Err(_) => panic!("The graph has cycles and cannot be organized as a DAG"),
        };

        // Determine the layers for each node
        let mut layers: HashMap<NodeIndex, usize> = HashMap::new();
        for node in &topo_sorted_nodes {
            let max_layer = graph
                .edges_directed(*node, petgraph::Direction::Incoming)
                .map(|e| *layers.get(&e.source()).unwrap_or(&0))
                .max()
                .unwrap_or(0);
            layers.insert(*node, max_layer + 1);
        }

        // Determine the maximum layer to set the canvas height
        let max_layer = *layers.values().max().unwrap_or(&0);

        // Group nodes by layers
        let mut layer_nodes: HashMap<usize, Vec<NodeIndex>> = HashMap::new();
        for (node, layer) in &layers {
            layer_nodes.entry(*layer).or_default().push(*node);
        }

        // Create SVG document
        let mut document = Document::new()
            .set("viewBox", (0, 0, 1000, (max_layer + 2) * 100))
            .set("width", "100%")
            .set("height", "100%");

        // Add title
        let title = Text::new()
            .set("x", 500)
            .set("y", 40)
            .set("text-anchor", "middle")
            .set("font-family", "Arial")
            .set("font-size", 30)
            .set("fill", "black")
            .add(svg::node::Text::new("Property DAG"));
        document = document.add(title);

        // Define node positions
        let mut positions = HashMap::new();
        for (layer, nodes) in layer_nodes.iter() {
            let y = *layer as f64 * 100.0 + 100.0;
            let num_nodes = nodes.len();
            let spacing = 1000.0 / (num_nodes + 1) as f64;
            for (i, node) in nodes.iter().enumerate() {
                let x = (i as f64 + 1.0) * spacing;
                positions.insert(*node, (x, y));
            }
        }

        // Add edges to SVG document
        for edge in graph.edge_references() {
            let source = edge.source();
            let target = edge.target();
            let (x1, y1) = positions[&source];
            let (x2, y2) = positions[&target];
            let line = Line::new()
                .set("x1", x1)
                .set("y1", y1)
                .set("x2", x2)
                .set("y2", y2)
                .set("stroke", "black");
            document = document.add(line);
        }

        // Add nodes to SVG document and count subscriptions
        let mut num_subscriptions = 0;
        for (node, &(x, y)) in &positions {
            let id = graph[*node].clone();
            let entry = sm.get(&PropertyId::from_string(&id)).unwrap();
            let subscriptions_count = entry.data.subscriptions.subscriptions.len();
            if subscriptions_count > 0 {
                num_subscriptions += subscriptions_count;
            }
            let fill_color = if subscriptions_count > 0 { "blue" } else { "red" };

            let circle = Circle::new()
                .set("cx", x)
                .set("cy", y)
                .set("r", 20)
                .set("fill", fill_color);
            let label = Text::new()
                .set("x", x)
                .set("y", y + 5.0)
                .set("text-anchor", "middle")
                .set("font-family", "Arial")
                .set("font-size", 15)
                .set("fill", "white")
                .add(svg::node::Text::new(id));
            document = document.add(circle).add(label);
        }

        // Add summary information at the bottom
        let num_nodes = graph.node_count();
        let num_edges = graph.edge_count();
        let summary = Text::new()
            .set("x", 500)
            .set("y", (max_layer + 2) as f64 * 100.0 - 20.0)
            .set("text-anchor", "middle")
            .set("font-family", "Arial")
            .set("font-size", 20)
            .set("fill", "black")
            .add(svg::node::Text::new(format!("Number of nodes: {}, Number of edges: {}, Number of subscriptions: {}", num_nodes, num_edges, num_subscriptions)));
        document = document.add(summary);

        // Write the SVG document to a file
        let path = Path::new(file_path);
        let mut file = File::create(&path).expect("Unable to create file");
        svg::write(&mut file, &document).expect("Unable to write SVG");
    }
}