use pax_language::{formatting::format_pax_template, from_pax, parse_pax_str, Pair, Rule};
use pax_runtime_api::PaxValue;

fn payloads(pair: Pair<Rule>, output: &mut Vec<String>) {
    match pair.as_rule() {
        Rule::body_text => {
            if let Some(value) = pax_language::text::decode_body_text(pair.as_str()).unwrap() {
                output.push(value);
            }
        }
        Rule::string => {
            let PaxValue::String(value) = from_pax(pair.as_str()).unwrap() else {
                panic!()
            };
            output.push(value);
        }
        _ => {
            for child in pair.into_inner() {
                payloads(child, output);
            }
        }
    }
}

fn values(source: &str) -> Vec<String> {
    let mut result = Vec::new();
    payloads(
        parse_pax_str(Rule::pax_component_definition, source).unwrap(),
        &mut result,
    );
    result.sort(); // Attribute order is deliberately canonicalized by the formatter.
    result
}

#[test]
fn bodies_and_raw_literals_survive_nested_formatting() {
    for source in [
        "<Text>Hello, world</Text>",
        "<Text>  inline  </Text>",
        "<Group><Text>  inline\r\n  continuation</Text></Group>",
        "<Group><Text>\r    block\r        continuation\r</Text></Group>",
        "<Group><Text>\n    one  \n        two\n\n    🚀é\n</Text></Group>",
        "<Group><Text>{r##\"\r\n  <Text>{x}</Text> & \\\\n \"#\r\n\"##}</Text></Group>",
        "<Text text={r#\"\n  literal |-DO_NOT_INSERT_TAB-|\n\"#} class=\"test\"/>",
        "<Text>123 &amp; true &#123; &#x1f680;</Text>",
        "<Text>\n\n    extra blank lines\n\n</Text>",
    ] {
        let expected = values(source);
        let once = format_pax_template(source.into()).unwrap();
        assert_eq!(values(&once), expected, "{source} => {once}");
        assert_eq!(format_pax_template(once.clone()).unwrap(), once, "{source}");
    }
}

#[test]
fn structure_and_prose_are_distinct() {
    for source in [
        "<Group>if self.visible { <Text>Hello</Text> }</Group>",
        "<Group>for item in self.items { <Text>{item}</Text> }</Group>",
        "<Group>slot(0)</Group>",
        "<Text>if you want to</Text>",
        "<Text>for everyone</Text>",
        "<Text>\"quoted words\" and `code`</Text>",
    ] {
        assert!(
            parse_pax_str(Rule::pax_component_definition, source).is_ok(),
            "{source}"
        );
    }
    for source in [
        "<Text>Hello {self.name}</Text>",
        "<Text>Hello <Button/></Text>",
        "<Text><Button/> world</Text>",
        "<Text>Hello</Button>",
        "<Text>{r#\"unfinished}</Text>",
    ] {
        assert!(
            parse_pax_str(Rule::pax_component_definition, source).is_err(),
            "{source}"
        );
    }
}

#[test]
fn raw_string_values_and_expression_display_roundtrip() {
    for value in [
        "",
        "  indented\n    continuation\n",
        "\r\n",
        "\\n",
        "\"### end",
        "<Text>{name}</Text>",
        "🚀",
    ] {
        let literal = pax_runtime_api::pax_value::string_literal(value);
        assert_eq!(from_pax(&literal).unwrap(), PaxValue::String(value.into()));
        let expression = pax_language::parse_pax_expression(&literal).unwrap();
        assert_eq!(
            from_pax(&expression.to_string()).unwrap(),
            PaxValue::String(value.into())
        );
    }
}
