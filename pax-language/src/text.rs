//! Scalar element-body text: layout whitespace and character references.

use std::{borrow::Cow, fmt};

/// Invalid character reference, with a byte offset in the original body.
#[derive(Debug, PartialEq, Eq)]
pub struct BodyTextError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for BodyTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at body byte {}", self.message, self.offset)
    }
}

impl std::error::Error for BodyTextError {}

fn reference(source: &str, offset: usize) -> Result<(char, usize), BodyTextError> {
    let invalid = || {
        BodyTextError {
        offset,
        message: "invalid text character reference; use &amp; for a literal &, or a raw string expression",
    }
    };
    let tail = &source[offset + 1..];
    // Stop at the next non-reference character, so malformed input never
    // rescans the remaining document looking for a distant semicolon.
    let end = tail
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '#')
        .ok_or_else(invalid)?;
    if tail.as_bytes()[end] != b';' {
        return Err(invalid());
    }
    let name = &tail[..end];
    let value = match name {
        "lt" => '<',
        "gt" => '>',
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        _ => {
            let (digits, radix) = if let Some(digits) = name.strip_prefix("#x") {
                (digits, 16)
            } else if let Some(digits) = name.strip_prefix('#') {
                (digits, 10)
            } else {
                return Err(invalid());
            };
            if digits.is_empty() {
                return Err(invalid());
            }
            u32::from_str_radix(digits, radix)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(invalid)?
        }
    };
    Ok((value, end + 2))
}

fn visit_references(
    source: &str,
    mut visit: impl FnMut(usize, char, usize),
) -> Result<(), BodyTextError> {
    let mut cursor = 0;
    while let Some(next) = source[cursor..].find('&') {
        let offset = cursor + next;
        let (value, len) = reference(source, offset)?;
        visit(offset, value, len);
        cursor = offset + len;
    }
    Ok(())
}

/// Remove block framing and common indentation without interpreting references.
/// Inline bodies retain their spaces. Line endings become LF; blank lines,
/// relative indentation and trailing spaces remain significant.
pub fn normalize_body_whitespace(source: &str) -> String {
    let source = if source.contains('\r') {
        Cow::Owned(source.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(source)
    };
    let horizontal = |s: &str| s.bytes().all(|b| matches!(b, b' ' | b'\t'));
    let Some((first, last)) = block_framing(&source) else {
        return source.into_owned();
    };
    let content = &source[first + 1..last];
    let mut prefix: Option<&str> = None;
    for line in content.split('\n').filter(|line| !horizontal(line)) {
        let len = line
            .bytes()
            .take_while(|b| matches!(b, b' ' | b'\t'))
            .count();
        let indent = &line[..len];
        prefix = Some(match prefix {
            None => indent,
            Some(prev) => {
                &prev[..prev
                    .bytes()
                    .zip(indent.bytes())
                    .take_while(|(a, b)| a == b)
                    .count()]
            }
        });
    }
    let prefix = prefix.unwrap_or("");
    let mut output = String::with_capacity(content.len());
    for (index, line) in content.split('\n').enumerate() {
        if index != 0 {
            output.push('\n');
        }
        output.push_str(line.strip_prefix(prefix).unwrap_or(line));
    }
    output
}

// Input has normalized LF line endings. Framing is a syntax distinction:
// normalization alone can also change an unframed CRLF body.
pub(crate) fn block_framing(source: &str) -> Option<(usize, usize)> {
    let first = source.find('\n')?;
    let last = source.rfind('\n')?;
    let horizontal = |s: &str| s.bytes().all(|b| matches!(b, b' ' | b'\t'));
    (first != last && horizontal(&source[..first]) && horizontal(&source[last + 1..]))
        .then_some((first, last))
}

/// Decode ordinary body text. Layout-only whitespace means no assignment.
/// References are decoded once, after whitespace normalization, and never
/// reinterpreted as template syntax. Raw string expressions bypass this path.
pub fn decode_body_text(source: &str) -> Result<Option<String>, BodyTextError> {
    if source
        .bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
    {
        return Ok(None);
    }
    // Validate before normalization to retain original diagnostic offsets.
    visit_references(source, |_, _, _| {})?;
    let normalized = normalize_body_whitespace(source);
    if !normalized.contains('&') {
        return Ok(Some(normalized));
    }
    let mut result = String::with_capacity(normalized.len());
    let mut cursor = 0;
    visit_references(&normalized, |offset, value, len| {
        result.push_str(&normalized[cursor..offset]);
        result.push(value);
        cursor = offset + len;
    })?;
    result.push_str(&normalized[cursor..]);
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_preserves_relative_and_markdown_whitespace() {
        assert_eq!(
            decode_body_text("\r\n\t  first  \r\n\t      second\r\n\r\n\t  third\r\n")
                .unwrap()
                .unwrap(),
            "first  \n    second\n\nthird"
        );
        assert_eq!(
            decode_body_text("  inline  ").unwrap().unwrap(),
            "  inline  "
        );
        assert_eq!(decode_body_text("\n    \n").unwrap(), None);
        assert_eq!(
            decode_body_text("\n\n  extra\n\n").unwrap().unwrap(),
            "\nextra\n"
        );
        assert_eq!(decode_body_text("\u{a0}é🚀").unwrap().unwrap(), "\u{a0}é🚀");
    }

    #[test]
    fn references_are_decoded_once_after_structure_and_whitespace() {
        assert_eq!(
            decode_body_text("&lt;Text&gt;&#123;name&#125;&amp;lt; &#x1F680; &#32;")
                .unwrap()
                .unwrap(),
            "<Text>{name}&lt; 🚀  "
        );
        assert_eq!(
            decode_body_text("&#10;line&#10;").unwrap().unwrap(),
            "\nline\n"
        );
        for invalid in [
            "&",
            "&bogus;",
            "&#xD800;",
            "&#1114112;",
            "&#;",
            "&amp next",
            "&amp&",
        ] {
            assert!(decode_body_text(invalid).is_err(), "{invalid}");
        }
        assert_eq!(decode_body_text("\r\n  &bad;").unwrap_err().offset, 4);
    }
}
