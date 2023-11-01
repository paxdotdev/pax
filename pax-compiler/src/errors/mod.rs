use std::{fmt::{self, Display}, error::Error};
use cargo_metadata::{Message, diagnostic::DiagnosticLevel};
use colored::*;
use std::io::Cursor;
use std::io::BufReader;
use color_eyre::eyre::{self, WrapErr, Report};
use color_eyre::Result;
use std::process::Output;
use regex::Regex;

pub mod source_map;
use crate::manifest::{Token, TokenType};

use self::source_map::SourceMap;


#[derive(Debug)]
pub struct PaxTemplateError {
    pub message: Option<String>,
    pub token: Token,
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

        let error_display = format!("Error: {}", error_message).red().bold();
        write!(f, "{}", error_display)?;

        // Optionally print the custom message below the error line if it's present
        if let Some(custom_message) = &self.message {
            write!(f, "\n{}", custom_message)?;
        }

        // Display the token location
        if let Some(loc) = &self.token.token_location {
            let location = format!(
                "\nLine {} : Col {}",
                loc.start_line_col.0, loc.start_line_col.1
            ).green();
            write!(f, "{}", location)?;

            // Check if there's a source_line and underline the issue in red
            if let Some(source_line) = &self.token.source_line {
                let underline = " ".repeat(loc.start_line_col.1) + 
                                &"^".repeat((loc.end_line_col.1 - loc.start_line_col.1).max(1));
                write!(f, "\n{}", source_line)?;
                write!(f, "\n{}", underline.red())?;
            }
        }

        Ok(())
    }
}



impl Error for PaxTemplateError {}

pub fn process_messages(output: Output, source_map: &SourceMap) -> Result<(), Report> {
    let stderr_stream = Cursor::new(output.stdout);
    let reader = BufReader::new(stderr_stream);

    let mut has_errors = false;

    for message in Message::parse_stream(reader) {
        if let Ok(Message::CompilerMessage(msg)) = message {
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
    // Typical type mismatch error given by rust c
    let re = Regex::new(r"the trait bound `([^:]+)::([^:]+): From<([^:]+)::([^>]+)>` is not satisfied").unwrap();

    if let Some(captures) = re.captures(&error) {
        let module1 = &captures[1];
        let type1 = &captures[2];
        let module2 = &captures[3];
        let type2 = &captures[4];

        return format!("Expected {}::{} but found {}::{}.", module1, type1, module2, type2);
    }

    // If the message doesn't match the expected format, return it as is
    error
}