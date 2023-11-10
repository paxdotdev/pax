//! # Errors Module
//!
//! The `errors` module provides structures and functions for throwing Pax related errors

use cargo_metadata::{diagnostic::DiagnosticLevel, Message};
use color_eyre::eyre::{self, eyre, Report};
use color_eyre::Result;
use colored::*;
use regex::Regex;
use std::io::BufReader;
use std::io::Cursor;
use std::process::Output;
use std::{
    error::Error,
    fmt::{self},
};

pub mod source_map;
use crate::manifest::{Token, TokenType};

use self::source_map::SourceMap;

/// PaxTemplateError is a custom error type for returning template errors.
/// Given a token and an optional custom message, it will display a user-friendly error message.

#[derive(Debug)]
pub struct PaxTemplateError {
    pub message: Option<String>,
    pub token: Token,
}

impl PaxTemplateError {
    pub fn new(message: Option<String>, token: Token) -> eyre::Report {
        let err = PaxTemplateError { message, token };
        eyre!(format!("{}", err))
    }

    fn underline_token(&self) -> String {
        let mut result = String::new();
        if let Some(loc) = &self.token.token_location {
            if loc.start_line_col.0 == loc.end_line_col.0 {
                if let Some(source_line) = &self.token.source_line {
                    let underline_len = (loc.end_line_col.1 - loc.start_line_col.1).max(1);
                    let underline = " ".repeat(loc.start_line_col.1) + &"^".repeat(underline_len);
                    result.push_str(&format!("\n{}", source_line));
                    result.push_str(&format!("\n{}", underline.bold().red()));
                    return result;
                }
            }
        }
        self.underline_raw_token()
    }

    fn underline_raw_token(&self) -> String {
        let raw_value_lines: Vec<&str> = self.token.raw_value.split('\n').collect();
        let mut result = String::new();

        for line in raw_value_lines.iter() {
            result.push_str(&format!("{}\n", line));
            let first_non_space = line.chars().position(|c| !c.is_whitespace()).unwrap_or(0);
            let underline = " ".repeat(first_non_space) + &"^".repeat(line.len() - first_non_space);
            result.push_str(&format!("{}\n", underline.red().bold()));
        }
        result
    }
}

impl fmt::Display for PaxTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Generate an error message based on the token type
        let error_message = match self.token.token_type {
            TokenType::Expression => "Invalid expression.",
            TokenType::Identifier => "Invalid identifier.",
            TokenType::LiteralValue => "Invalid literal value.",
            TokenType::IfExpression => "Invalid 'if' expression.",
            TokenType::ForPredicate => "Invalid 'for' predicate.",
            TokenType::ForSource => "Invalid 'for' source.",
            TokenType::SlotExpression => "Invalid slot expression.",
            TokenType::EventId => "Invalid event ID.",
            TokenType::Handler => "Invalid handler.",
            TokenType::SettingKey => "Invalid setting key.",
            TokenType::Selector => "Invalid selector.",
            TokenType::PascalIdentifier => "Invalid type",
            TokenType::Unknown => "Unknown token error.",
        };

        let error_display = format!("Error: {}", error_message).bold().red();
        write!(f, "\n{}", error_display)?;

        // Display the token location
        if let Some(loc) = &self.token.token_location {
            let location = format!(
                "\n\nLine {} : Col {}",
                loc.start_line_col.0, loc.start_line_col.1
            )
            .green();
            write!(f, "{}", location)?;

            // Underline the issue based on the line information
            let token_underline = self.underline_token();
            write!(f, "\n{}", token_underline)?;
        }

        // Optionally print the custom message below the error line if it's present
        if let Some(custom_message) = &self.message {
            let formatted_custom_message = (*custom_message).bold().red();
            write!(f, "\n{}\n", formatted_custom_message)?;
        }

        Ok(())
    }
}

impl Error for PaxTemplateError {}

pub fn process_messages(output: Output, source_map: &SourceMap, verbose: bool) -> Result<(), Report> {
    let stderr_stream = Cursor::new(output.stdout);
    let reader = BufReader::new(stderr_stream);

    let mut has_errors = false;

    for message in Message::parse_stream(reader) {
        if let Ok(Message::CompilerMessage(msg)) = message {
            if verbose {
                eprintln!("{:?}", msg.message);
            }
            if msg.message.level == DiagnosticLevel::Error && !msg.message.spans.is_empty() {
                let line = msg.message.spans[0].line_start;
                if let Some(range_data) = source_map.get_range_for_line(line) {
                    let current_error_msg = transform_error_message(msg.message.message);
                    let error_display = PaxTemplateError {
                        message: Some(current_error_msg),
                        token: range_data.token.clone(),
                    };
                    eprintln!("{}", error_display);
                    has_errors = true;
                }
            }
        }
    }

    if has_errors {
        Err(color_eyre::eyre::eyre!("Failed to compile Pax Template"))
    } else {
        Ok(())
    }
}

// Transforms the rust trait message from underlying code gen into relevant user-facing error
fn transform_error_message(error: String) -> String {
    // Typical type mismatch error given by rustc
    let re =
        Regex::new(r"the trait bound `([^:]+)::([^:]+): From<([^:]+)::([^>]+)>` is not satisfied")
            .unwrap();

    if let Some(captures) = re.captures(&error) {
        let module1 = &captures[1];
        let type1 = &captures[2];
        let module2 = &captures[3];
        let type2 = &captures[4];

        return format!(
            "Expected {}::{} but found {}::{}.",
            module1, type1, module2, type2
        );
    }

    // If the message doesn't match the expected format, return it as is
    error
}
