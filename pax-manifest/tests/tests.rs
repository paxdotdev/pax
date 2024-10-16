#[cfg(test)]
#[cfg(feature = "parsing")]
mod tests {

    use pax_manifest::{utils, ValueDefinition};

    #[test]
    fn test_parse_empty() {
        assert!(matches!(utils::parse_value(""), Ok(None)));
    }

    #[test]
    fn test_parse_identifier() {
        let res = utils::parse_value("identifier");
        if let Ok(Some(ValueDefinition::Identifier(ident))) = res {
            assert_eq!(&ident.name, "identifier");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_literal_number() {
        let res = utils::parse_value("5");
        if let Ok(Some(ValueDefinition::LiteralValue(pv))) = res {
            assert_eq!(&pv.to_string(), "5");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_expression() {
        let res = utils::parse_value("{5 + 3}");
        if let Ok(Some(ValueDefinition::Expression(info))) = res {
            assert_eq!(&info.expression.to_string(), "5 + 3");
        } else {
            panic!("unexpected result: {:?}", res);
        }
    }

    #[test]
    fn test_parse_with_extra() {
        let res = utils::parse_value("{5 + 3}this_shouldn't succeed");
        assert!(matches!(res, Err(_)));
    }
}
