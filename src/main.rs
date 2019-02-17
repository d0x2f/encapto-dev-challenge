extern crate csv;
extern crate petgraph;
extern crate regex;
extern crate rpn;
#[macro_use]
extern crate lazy_static;
extern crate itertools;

use csv::ReaderBuilder;
use csv::StringRecord;
use itertools::Itertools;
use petgraph::algo::has_path_connecting;
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::NodeIndex;
use petgraph::Directed;
use petgraph::Graph;
use regex::Regex;
use std::char;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::error::Error;

type Map = BTreeMap<(usize, usize), String>;

/// Read a CSV file in from the given filename and parse it's contents into a map.
fn parse_csv(filename: String) -> Result<Map, Box<Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(filename)?;
    let mut map = BTreeMap::new();

    for (i, result) in reader.records().enumerate() {
        map.extend(read_record(i, &result?));
    }
    Ok(map)
}

/// Read a CSV StringRecord and parse it into a map representing given row index.
fn read_record(row: usize, record: &StringRecord) -> Map {
    let mut map = Map::new();
    for (column, field) in record.iter().enumerate() {
        map.insert((row, column), String::from(field));
    }
    map
}

/// Convert a map index to a cell reference string, e.g. (3, 5) => "f4".
fn index_to_cell_reference((row, column): &(usize, usize)) -> Result<String, String> {
    let column_name = match char::from_digit(*column as u32 + 10, 36) {
        Some(column_name) => column_name,
        None => return Err("Index out of bounds".to_string()),
    };
    let mut cell_reference = column_name.to_string();
    cell_reference.push_str(&(row + 1).to_string());
    Ok(cell_reference)
}

/// Convert a cell reference string to a map index, e.g. "e7" => (6, 4).
fn cell_reference_to_index(cell_reference: &str) -> Result<(usize, usize), String> {
    let attempt = |cell_reference: &str| -> Option<(usize, usize)> {
        let column_char = cell_reference.to_ascii_lowercase().chars().next()?;
        let column = char::to_digit(column_char, 36)? as usize;
        let row = match cell_reference[1..].parse::<usize>() {
            Ok(n) => n,
            Err(_) => return None,
        };
        if row == 0 || column < 10 {
            return None;
        }
        Some((row - 1, column - 10))
    };

    match attempt(cell_reference) {
        Some(index) => Ok(index),
        None => Err(format!(
            "Unable to parse cell reference: {}",
            cell_reference
        )),
    }
}

/// Construct a directed graph representing the map, then prune the nodes not connected to
/// the node of interest.
/// Return if the resultant graph contains a cycle.
fn detect_cycle(map: &Map, cell_index: &(usize, usize)) -> Result<bool, String> {
    let mut nodes = HashMap::<String, NodeIndex<u32>>::new();
    let mut graph = Graph::<String, u32, Directed>::new();

    // First pass to create the nodes
    for (index, _) in map {
        let cell_reference = index_to_cell_reference(index)?;
        let node = graph.add_node(cell_reference.clone());
        nodes.insert(cell_reference, node);
    }

    // Second pass to create the edges
    for (index, expression) in map {
        let current_node = nodes[&index_to_cell_reference(index)?];
        for (child_cell_reference, _) in extract_cell_references_with_indexes(expression.as_str())?
        {
            let child_node = nodes[child_cell_reference];
            graph.add_edge(current_node, child_node, 1);
        }
    }

    // Third pass to prune edges not related to the node in question
    let current_node = nodes[&index_to_cell_reference(cell_index)?];
    let mut pruned_graph = graph.clone();
    pruned_graph.retain_nodes(|_, n| has_path_connecting(&graph, current_node, n, None));

    Ok(is_cyclic_directed(&graph))
}

/// Evaluate the given cell recusing as neccessary to produce a single value.
/// This function assumes that there is no cycles in the references (i.e. that detect_cycle returns false).
fn evaluate_recursive(
    map: &Map,
    solved_map: &mut Map,
    cell_index: &(usize, usize),
) -> Result<String, String> {
    let expression = map
        .get(cell_index)
        .ok_or("Invalid cell reference".to_string())?
        .to_owned();

    if expression.trim().is_empty() {
        solved_map.insert(cell_index.to_owned(), "0".to_string());
        return Ok("0".to_string());
    }
    let children = extract_cell_references_with_indexes(expression.as_str())?;
    let mut resolved_expression = expression.clone();

    for (child_cell_reference, child_index) in children {
        let child_result = evaluate_recursive(map, solved_map, &child_index)?;
        resolved_expression =
            resolved_expression.replace(child_cell_reference, child_result.as_str());
    }
    match rpn::evaluate(&resolved_expression) {
        Ok(f) => {
            let result = f.to_string();
            solved_map.insert(cell_index.to_owned(), result);
            Ok(f.to_string())
        }
        Err(_) => {
            solved_map.insert(cell_index.to_owned(), "#ERR".to_string());
            Err(format!(
                "Unable to evaluate expression: {}",
                resolved_expression
            ))
        }
    }
}

/// Parse input arguments.
/// Expecting only one argument to be given which will be used as the filename.
fn parse_arguments() -> Result<String, String> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        Err("No filename argument given.".to_string())
    } else {
        Ok(args.remove(1))
    }
}

/// Extract any cell references from a string,
/// Then convert the string reference to an integer tuple.
///  e.g. "1 a4 + 14 - h2" => [("a4", (3,0)), ("h2", (1, 7))]
fn extract_cell_references_with_indexes(
    string: &str,
) -> Result<Vec<(&str, (usize, usize))>, String> {
    lazy_static! {
        static ref RE: Regex = Regex::new("([a-zA-Z]\\d+)").unwrap();
    }

    Ok(RE
        .captures_iter(string)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str())
        .map(|m| (m, cell_reference_to_index(m).unwrap()))
        .collect::<Vec<_>>())
}

/// Print the given map in CSV format.
fn print_csv(map: &Map) {
    let rows = map.iter().group_by(|v| (v.0).0);
    for (_key, group) in &rows {
        let row = group.map(|v| v.1.to_owned()).collect::<Vec<_>>().join(",");
        println!("{}", row);
    }
}

/// Load the CSV file into a map, evaluate each cell, print the result as CSV.
fn main() {
    let attempt = || -> Result<(), String> {
        // Load csv file
        let filename = parse_arguments()?;
        let map = match parse_csv(filename) {
            Ok(map) => map,
            Err(error) => return Err(error.description().to_string()),
        };

        // Evaluate each cell
        let mut solved_map = Map::new();
        for (index, _) in &map {
            if detect_cycle(&map, index)? {
                eprintln!("Cycle detected.");
                solved_map.insert(*index, "#ERR".to_string());
            } else {
                match evaluate_recursive(&map, &mut solved_map, index) {
                    Ok(_) => (),
                    Err(e) => eprintln!("{}", e),
                };
            }
        }
        // Print the solution as csv
        print_csv(&solved_map);
        Ok(())
    };

    if let Err(error) = attempt() {
        eprintln!("{}", error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_to_cell_reference_success() {
        assert_eq!(index_to_cell_reference(&(1, 4)), Ok("e2".to_string()));
        assert_eq!(index_to_cell_reference(&(3, 5)), Ok("f4".to_string()));
    }

    #[test]
    fn index_to_cell_reference_failure() {
        assert_eq!(
            index_to_cell_reference(&(1, 26)),
            Err("Index out of bounds".to_string())
        );
    }

    #[test]
    fn cell_reference_to_index_success() {
        assert_eq!(cell_reference_to_index("e7"), Ok((6, 4)));
        assert_eq!(cell_reference_to_index("u14"), Ok((13, 20)));
    }

    #[test]
    fn cell_reference_to_index_failure() {
        assert_eq!(
            cell_reference_to_index("!7"),
            Err("Unable to parse cell reference: !7".to_string())
        );
        assert_eq!(
            cell_reference_to_index("a0"),
            Err("Unable to parse cell reference: a0".to_string())
        );
        assert_eq!(
            cell_reference_to_index("10"),
            Err("Unable to parse cell reference: 10".to_string())
        );
    }
}
