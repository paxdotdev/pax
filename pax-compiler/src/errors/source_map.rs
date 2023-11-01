use std::{collections::{HashMap, BTreeMap}, fs::{File,OpenOptions}, io::{BufReader, BufRead}};
use regex::Regex;
use std::io::Write;
use crate::{manifest::Token, templating::MappedString};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RangeData {
    pub end: usize,
    pub id: usize,
    pub token: Token,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceMap {
    pub sources: HashMap<usize, Token>,
    pub ranges: BTreeMap<usize, RangeData>,
    pub next_id: usize,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap {
            sources: HashMap::new(),
            ranges: BTreeMap::new(),
            next_id: 0,
        }
    }

    pub fn generate_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
    
    pub fn insert(&mut self, token: Token) -> usize {
        let id = self.generate_id();
        self.sources.insert(id, token);
        id
    }

    pub fn generate_start_end_marker(&mut self, id: usize) -> (String, String) {
        let start_marker = format!("/* source_map_start_{} */", id);
        let end_marker = format!("/* source_map_end_{} */", id);
        (start_marker, end_marker)
    }

    pub fn generate_mapped_string(&mut self, content: String, id: usize) -> MappedString {
        let (start_marker, end_marker) = self.generate_start_end_marker(id);
        MappedString {
            content,
            source_map_start_marker: Some(start_marker),
            source_map_end_marker: Some(end_marker),
        }
    }
    
    pub fn extract_ranges_from_generated_code(&mut self, file_path: &str) {
        let file = File::open(file_path).expect("Failed to open file");
        let reader = BufReader::new(file);
    
        let start_regex = Regex::new(r"/\* source_map_start_(\d+) \*/").unwrap();
        let end_regex = Regex::new(r"/\* source_map_end_(\d+) \*/").unwrap();
    
        let mut start_positions: HashMap<usize, usize> = HashMap::new();
        let mut processed_content = String::new();
    
        for (line_num, res) in reader.lines().enumerate() {
            let line = res.expect("Invalid line");
            let mut line_processed = line.clone();
            
            if let Some(captures) = start_regex.captures(&line) {
                let id: usize = captures[1].parse().unwrap();
                let start_pos = line_num + 1;
                start_positions.insert(id, start_pos);
                line_processed = line_processed.replace(&captures[0], "");
            }
            
            if let Some(captures) = end_regex.captures(&line) {
                let id: usize = captures[1].parse().unwrap();
                if let Some(start_pos) = start_positions.remove(&id) {
                    let end_pos = line_num + 1;
                    let token = self.sources.get(&id).cloned().unwrap_or_else(|| Token::default());
                    let range_data = RangeData { end: end_pos, id, token };
                    self.ranges.insert(start_pos, range_data);
                }
                line_processed = line_processed.replace(&captures[0], "");
            }
    
            processed_content.push_str(&line_processed);
            processed_content.push('\n');
        }
    
        let mut file = OpenOptions::new().write(true).truncate(true).open(file_path).expect("Failed to open file in write mode");
        write!(file, "{}", processed_content).expect("Failed to write to file");
    }

    pub fn get_range_for_line(&self, line: usize) -> Option<&RangeData> {
        let (_, range) = self.ranges.range(..=line).next_back()?;
        if line <= range.end {
            Some(range)
        } else {
            None
        }
    }
}
