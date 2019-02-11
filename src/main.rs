extern crate csv;
extern crate petgraph;
extern crate regex;
#[macro_use]
extern crate lazy_static;

use csv::ReaderBuilder;
use csv::StringRecord;
use petgraph::Directed;
use petgraph::Graph;
use regex::Regex;
use std::char;
use std::collections::HashMap;
use std::env;
use std::error::Error;

type Map = HashMap<(usize, usize), String>;

fn parse_csv(filename: String) -> Result<Map, Box<Error>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(filename)?;
    let mut map = HashMap::new();

    for (i, result) in reader.records().enumerate() {
        map.extend(read_record(i, &result?));
    }
    Ok(map)
}

fn read_record(row: usize, record: &StringRecord) -> Map {
    let mut map = Map::new();
    for (column, field) in record.iter().enumerate() {
        map.insert((row, column), String::from(field));
    }
    map
}

// fn build_graph_partition(
//     map: &Map,
//     cell: (usize, usize),
// ) -> Result<Graph<&(usize, usize), u32, Directed>, String> {
//     let expression = map.get(&cell).unwrap();
//     let mut graph = Graph::<&(usize, usize), u32, Directed>::new();
//     let parent_node = graph.add_node(&cell);
//     let children = extract_cell_references(expression)?
//         .into_iter()
//         .map(|c| cell_reference_to_index(c).unwrap());
//     for child in children {
//         let child_node = graph.add_node(child);
//         graph.add_edge(parent_node, child_node, 1);
//     }

//     Err("not implemented".to_string())
// }

fn evaluate_reverse_polish(input: &str) {}

fn parse_arguments() -> Result<String, String> {
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        Err("No filename argument given.".to_string())
    } else {
        Ok(args.remove(1))
    }
}

/// Convert a map index to a cell reference string, e.g. (3, 5) => "f4".
fn index_to_cell_reference((row, column): (usize, usize)) -> Result<String, String> {
    let column_name = match char::from_digit(column as u32 + 10, 36) {
        Some(column_name) => column_name,
        None => return Err("Index out of bounds".to_string()),
    };
    let mut cell_reference = column_name.to_string();
    cell_reference.push_str(&(row + 1).to_string());
    Ok(cell_reference)
}

// Convert a cell reference string to a map index, e.g. "e7" => (6, 4).
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

/// Extract any cell references from a string, e.g. "1 a4 + 14 - h2" => ["a4", "h2"]
fn extract_cell_references(string: &str) -> Result<Vec<&str>, String> {
    lazy_static! {
        static ref RE: Regex = Regex::new("([a-zA-Z]\\d+)").unwrap();
    }

    Ok(RE
        .captures_iter(string)
        .filter_map(|cap| cap.get(1))
        .map(|m| m.as_str())
        .collect::<Vec<_>>())
}

fn main() {
    let attempt = || -> Result<(), String> {
        let filename = parse_arguments()?;
        let map = match parse_csv(filename) {
            Ok(map) => map,
            Err(error) => return Err(error.description().to_string()),
        };
        for ((row, column), expression) in &map {
            // let graph = build_graph_partition(&map, (*row, *column));
            // check for cycles
            // evaluate expressions bottom up.
            // find and replace cell references with evaluated values (or #ERR on error)
            println!("({}, {}) => {}", row, column, expression);
        }
        // Print final map as csv
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
        assert_eq!(index_to_cell_reference((1, 4)), Ok("e2".to_string()));
        assert_eq!(index_to_cell_reference((3, 5)), Ok("f4".to_string()));
    }

    #[test]
    fn index_to_cell_reference_failure() {
        assert_eq!(
            index_to_cell_reference((1, 26)),
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
