#!/usr/bin/env python3
"""Generate the Pax docs highlighter from pax-language's Pest grammar.

The generated JS intentionally uses a tiny deterministic scanner instead of a
highlight.js regex grammar. Pest is a parser grammar and highlight.js is a regex
mode engine; trying to translate one into the other directly gets brittle around
nested template expressions, settings blocks, and timelines. This generator still
uses pax.pest as the token-vocabulary source of truth.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_GRAMMAR = ROOT / "pax-language" / "src" / "pax.pest"
DEFAULT_OUTPUT = ROOT / "pax-docs" / "book" / "theme" / "highlight-pax.js"

IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_-]*$")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--grammar", type=Path, default=DEFAULT_GRAMMAR)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()

    grammar = args.grammar.read_text()
    tokens = extract_tokens(grammar)
    source_hash = hashlib.sha256(grammar.encode()).hexdigest()[:16]
    rendered = render_highlighter(tokens, source_hash, args.grammar, args.output)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    existing = args.output.read_text() if args.output.exists() else None
    if existing != rendered:
        args.output.write_text(rendered)
        print(f"updated {args.output}")
    else:
        print(f"unchanged {args.output}")


def extract_tokens(grammar: str) -> dict[str, list[str]]:
    grammar_without_comments = strip_pest_comments(grammar)
    all_literals = extract_string_literals(grammar_without_comments)
    color_constants = sorted(
        literal
        for literal in extract_rule_literals(grammar, "literal_color_const")
        if literal.isupper()
    )
    units = sorted(
        set(
            literal
            for literal in extract_rule_literals(grammar, "literal_number_unit")
            if literal not in {""}
        )
    )
    color_functions = sorted(
        set(
            literal[:-1]
            for literal in all_literals
            if literal.endswith("(") and literal[:-1] in {"rgb", "rgba", "hsl", "hsla"}
        )
    )

    literal_words = set(
        literal
        for literal in all_literals
        if IDENT_RE.match(literal) and not literal.isupper()
    )
    boolean_literals = set(extract_rule_literals(grammar, "literal_boolean"))
    option_literals = {"Some", "None", "Option"}.intersection(literal_words)
    if "Option::" in all_literals:
        option_literals.add("Option")
    keywords = sorted(
        {word for word in literal_words if len(word) > 1 and word != "_"}
        - boolean_literals
        - option_literals
        - set(color_functions)
        - set(units)
    )

    return {
        "keywords": keywords,
        "boolean_literals": sorted(boolean_literals),
        "option_literals": sorted(option_literals),
        "color_constants": color_constants,
        "color_functions": color_functions,
        "units": units,
    }


def strip_pest_comments(text: str) -> str:
    out: list[str] = []
    index = 0
    state = "code"
    escape = False

    while index < len(text):
        ch = text[index]
        nxt = text[index : index + 2]

        if state == "line_comment":
            if ch == "\n":
                state = "code"
                out.append(ch)
            index += 1
            continue

        if state == "block_comment":
            if nxt == "*/":
                state = "code"
                index += 2
            else:
                if ch == "\n":
                    out.append(ch)
                index += 1
            continue

        if state == "string":
            out.append(ch)
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                state = "code"
            index += 1
            continue

        if nxt == "//":
            state = "line_comment"
            index += 2
            continue
        if nxt == "/*":
            state = "block_comment"
            index += 2
            continue
        if ch == '"':
            state = "string"
        out.append(ch)
        index += 1

    return "".join(out)


def extract_rule_literals(grammar: str, rule_name: str) -> list[str]:
    body = find_rule_body(grammar, rule_name)
    return extract_string_literals(body)


def extract_string_literals(text: str) -> list[str]:
    values = []
    for match in re.finditer(r'"((?:\\.|[^"\\])*)"', text):
        value = bytes(match.group(1), "utf-8").decode("unicode_escape")
        values.append(value)
    return values


def find_rule_body(grammar: str, rule_name: str) -> str:
    match = re.search(rf"(?m)^\s*{re.escape(rule_name)}\s*=\s*[@_$]?\s*{{", grammar)
    if not match:
        return ""

    start = grammar.find("{", match.end() - 1)
    index = start
    depth = 0
    state = "code"
    escape = False

    while index < len(grammar):
        ch = grammar[index]
        nxt = grammar[index : index + 2]

        if state == "line_comment":
            if ch == "\n":
                state = "code"
            index += 1
            continue

        if state == "block_comment":
            if nxt == "*/":
                state = "code"
                index += 2
            else:
                index += 1
            continue

        if state == "string":
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                state = "code"
            index += 1
            continue

        if nxt == "//":
            state = "line_comment"
            index += 2
            continue
        if nxt == "/*":
            state = "block_comment"
            index += 2
            continue
        if ch == '"':
            state = "string"
            index += 1
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return grammar[start + 1 : index]
        index += 1

    return ""


def render_highlighter(
    tokens: dict[str, list[str]], source_hash: str, grammar_path: Path, output_path: Path
) -> str:
    grammar_label = grammar_path.relative_to(ROOT).as_posix()
    output_label = output_path.relative_to(ROOT).as_posix()

    def js(value: object) -> str:
        return json.dumps(value, indent=4)

    return fr'''// Generated by pax-docs/scripts/generate_pax_highlight.py.
// Source grammar: {grammar_label} sha256:{source_hash}
// Output: {output_label}
(function () {{
    const PAX_GRAMMAR_TOKENS = {{
        keywords: {js(tokens["keywords"])},
        booleanLiterals: {js(tokens["boolean_literals"])},
        optionLiterals: {js(tokens["option_literals"])},
        colorConstants: {js(tokens["color_constants"])},
        colorFunctions: {js(tokens["color_functions"])},
        units: {js(tokens["units"])}
    }};

    const KEYWORDS = new Set(PAX_GRAMMAR_TOKENS.keywords);
    const LITERALS = new Set(
        PAX_GRAMMAR_TOKENS.booleanLiterals
            .concat(PAX_GRAMMAR_TOKENS.optionLiterals)
            .concat(PAX_GRAMMAR_TOKENS.colorConstants)
    );
    const COLOR_FUNCTIONS = new Set(PAX_GRAMMAR_TOKENS.colorFunctions);
    const UNITS = new Set(PAX_GRAMMAR_TOKENS.units.filter((unit) => /^[A-Za-z]+$/.test(unit)));
    const OPERATORS = ["==", "!=", ">=", "<=", "&&", "||", "%%", "??", "..", "::", "+", "-", "*", "/", "%", "^", "?", ":", "=", ">", "<", "!", ",", ".", "[", "]", "(", ")"];

    function escapeHtml(value) {{
        return value
            .replace(/&/g, "&amp;")
            .replace(/</g, "&lt;")
            .replace(/>/g, "&gt;")
            .replace(/"/g, "&quot;")
            .replace(/'/g, "&#x27;");
    }}

    function span(className, value) {{
        return `<span class="hljs-${{className}}">${{escapeHtml(value)}}</span>`;
    }}

    function isWhitespace(ch) {{
        return ch === " " || ch === "\t" || ch === "\n" || ch === "\r";
    }}

    function isIdentStart(ch) {{
        return !!ch && /[A-Za-z_]/.test(ch);
    }}

    function isIdentPart(ch) {{
        return !!ch && /[A-Za-z0-9_-]/.test(ch);
    }}

    function readIdentifier(source, index) {{
        if (!isIdentStart(source[index])) {{
            return null;
        }}
        let end = index + 1;
        while (end < source.length && isIdentPart(source[end])) {{
            end += 1;
        }}
        return {{ text: source.slice(index, end), end }};
    }}

    function nextNonWhitespace(source, index) {{
        let cursor = index;
        while (cursor < source.length && isWhitespace(source[cursor])) {{
            cursor += 1;
        }}
        return cursor;
    }}

    function readString(source, index) {{
        const quote = source[index];
        let cursor = index + 1;
        let escaped = false;
        while (cursor < source.length) {{
            const ch = source[cursor];
            cursor += 1;
            if (escaped) {{
                escaped = false;
            }} else if (ch === "\\") {{
                escaped = true;
            }} else if (ch === quote) {{
                break;
            }}
        }}
        return {{ html: span("string", source.slice(index, cursor)), index: cursor }};
    }}

    function readComment(source, index) {{
        if (source.startsWith("//", index)) {{
            let end = source.indexOf("\n", index + 2);
            if (end === -1) {{
                end = source.length;
            }} else {{
                end += 1;
            }}
            return {{ html: span("comment", source.slice(index, end)), index: end }};
        }}
        if (source.startsWith("/*", index)) {{
            const close = source.indexOf("*/", index + 2);
            const end = close === -1 ? source.length : close + 2;
            return {{ html: span("comment", source.slice(index, end)), index: end }};
        }}
        if (source.startsWith("<!--", index)) {{
            const close = source.indexOf("-->", index + 4);
            const end = close === -1 ? source.length : close + 3;
            return {{ html: span("comment", source.slice(index, end)), index: end }};
        }}
        return null;
    }}

    function readNumber(source, index) {{
        const match = source.slice(index).match(/^-?(?:(?:\d+(?:\.\d+)?)|(?:\.\d+))(?:%|px|deg|rad)?/);
        if (!match) {{
            return null;
        }}
        const before = index > 0 ? source[index - 1] : "";
        const after = source[index + match[0].length] || "";
        if ((/[A-Za-z0-9_]/.test(before) && source[index] !== "-") || /[A-Za-z0-9_]/.test(after)) {{
            return null;
        }}
        return {{ html: span("number", match[0]), index: index + match[0].length }};
    }}

    function readAtDirective(source, index) {{
        const ident = readIdentifier(source, index + 1);
        if (!ident) {{
            return null;
        }}
        let html = span("meta", source.slice(index, ident.end));
        let cursor = ident.end;
        if (ident.text === "timeline") {{
            const name = readIdentifier(source, nextNonWhitespace(source, cursor));
            if (name) {{
                html += escapeHtml(source.slice(cursor, name.end).replace(name.text, ""));
                html += span("title", name.text);
                cursor = name.end;
            }}
        }}
        return {{ html, index: cursor }};
    }}

    function readSelector(source, index) {{
        const prefix = source[index];
        if (prefix !== "#" && prefix !== ".") {{
            return null;
        }}
        const ident = readIdentifier(source, index + 1);
        if (!ident) {{
            return null;
        }}
        return {{
            html: span(prefix === "#" ? "selector-id" : "selector-class", source.slice(index, ident.end)),
            index: ident.end
        }};
    }}

    function readOperator(source, index, allowed) {{
        const operators = allowed || OPERATORS;
        for (const op of operators) {{
            if (source.startsWith(op, index)) {{
                return {{ html: span("operator", op), index: index + op.length }};
            }}
        }}
        return null;
    }}

    function classifyIdentifier(source, index, context) {{
        const first = readIdentifier(source, index);
        if (!first) {{
            return null;
        }}

        let cursor = first.end;
        const pathParts = [first.text];
        while (source.startsWith("::", cursor)) {{
            const next = readIdentifier(source, cursor + 2);
            if (!next) {{
                break;
            }}
            pathParts.push(next.text);
            cursor = next.end;
        }}
        if (pathParts.length > 1) {{
            let html = "";
            pathParts.forEach((part, partIndex) => {{
                if (partIndex > 0) {{
                    html += span("operator", "::");
                }}
                html += span("symbol", part);
            }});
            return {{ html, index: cursor }};
        }}

        const name = first.text;
        const lookahead = nextNonWhitespace(source, first.end);
        const ch = source[lookahead];
        const nextTwo = source.slice(lookahead, lookahead + 2);
        if (context && context.tag && (ch === "=" || ch === ":") && nextTwo !== "==" && nextTwo !== "::") {{
            return {{ html: span("attr", name), index: first.end }};
        }}
        if ((!context || !context.expression) && (ch === "=" || ch === ":") && nextTwo !== "==" && nextTwo !== "::") {{
            return {{ html: span("attr", name), index: first.end }};
        }}
        if (COLOR_FUNCTIONS.has(name) && ch === "(") {{
            return {{ html: span("built_in", name), index: first.end }};
        }}
        if (KEYWORDS.has(name)) {{
            return {{ html: span("keyword", name), index: first.end }};
        }}
        if (LITERALS.has(name)) {{
            return {{ html: span("literal", name), index: first.end }};
        }}
        if (UNITS.has(name)) {{
            return {{ html: span("number", name), index: first.end }};
        }}
        if (ch === "(") {{
            return {{ html: span("title", name), index: first.end }};
        }}
        if (/^[A-Z]/.test(name) && /[a-z]/.test(name)) {{
            return {{ html: span(ch === "{{" ? "type" : "symbol", name), index: first.end }};
        }}
        return {{ html: span("variable", name), index: first.end }};
    }}

    function scanBracedExpression(source, index) {{
        let html = `<span class="hljs-template-variable">${{escapeHtml(source[index])}}`;
        let cursor = index + 1;
        while (cursor < source.length) {{
            const ch = source[cursor];
            if (ch === "}}") {{
                html += escapeHtml(ch) + "</span>";
                return {{ html, index: cursor + 1 }};
            }}
            if (ch === "{{") {{
                const nested = scanBracedExpression(source, cursor);
                html += nested.html;
                cursor = nested.index;
                continue;
            }}
            const token = scanToken(source, cursor, {{ expression: true }});
            html += token.html;
            cursor = token.index;
        }}
        html += "</span>";
        return {{ html, index: cursor }};
    }}

    function scanTag(source, index) {{
        let cursor = index;
        let html = "";
        if (source.startsWith("</", cursor)) {{
            html += span("tag", "</");
            cursor += 2;
        }} else {{
            html += span("tag", "<");
            cursor += 1;
        }}

        const name = readIdentifier(source, cursor);
        if (name) {{
            html += span("name", name.text);
            cursor = name.end;
        }}

        while (cursor < source.length) {{
            if (source.startsWith("/>", cursor)) {{
                html += span("tag", "/>");
                return {{ html, index: cursor + 2 }};
            }}
            if (source[cursor] === ">") {{
                html += span("tag", ">");
                return {{ html, index: cursor + 1 }};
            }}
            if (source[cursor] === "{{") {{
                const expr = scanBracedExpression(source, cursor);
                html += expr.html;
                cursor = expr.index;
                continue;
            }}
            if (source[cursor] === "@") {{
                const directive = readAtDirective(source, cursor);
                if (directive) {{
                    html += directive.html;
                    cursor = directive.index;
                    continue;
                }}
            }}
            const token = scanToken(source, cursor, {{ tag: true }});
            html += token.html;
            cursor = token.index;
        }}
        return {{ html, index: cursor }};
    }}

    function scanToken(source, index, context) {{
        const ch = source[index];
        const comment = readComment(source, index);
        if (comment) {{
            return comment;
        }}
        if (ch === '"' || ch === "'" || ch === "`") {{
            return readString(source, index);
        }}
        if (isWhitespace(ch)) {{
            return {{ html: escapeHtml(ch), index: index + 1 }};
        }}
        if (ch === "@") {{
            const directive = readAtDirective(source, index);
            if (directive) {{
                return directive;
            }}
        }}
        if ((ch === "#" || ch === ".") && !(context && context.expression)) {{
            const selector = readSelector(source, index);
            if (selector) {{
                return selector;
            }}
        }}
        const number = readNumber(source, index);
        if (number) {{
            return number;
        }}
        if (isIdentStart(ch)) {{
            const ident = classifyIdentifier(source, index, context || {{}});
            if (ident) {{
                return ident;
            }}
        }}
        const operator = readOperator(source, index, context && context.tag ? ["="] : undefined);
        if (operator) {{
            return operator;
        }}
        return {{ html: escapeHtml(ch), index: index + 1 }};
    }}

    function highlightPax(source) {{
        let html = "";
        let index = 0;
        while (index < source.length) {{
            if (source[index] === "<" && /<\/?[A-Z]/.test(source.slice(index, index + 3))) {{
                const tag = scanTag(source, index);
                html += tag.html;
                index = tag.index;
                continue;
            }}
            const token = scanToken(source, index, {{}});
            html += token.html;
            index = token.index;
        }}
        return html;
    }}

    function isPaxCode(code) {{
        return code && (
            code.classList.contains("language-pax") ||
            code.classList.contains("language-paxel") ||
            code.classList.contains("lang-pax") ||
            code.classList.contains("lang-paxel")
        );
    }}

    function resetCode(code) {{
        const source = code.textContent;
        code.textContent = source;
        code.classList.remove("no-highlight");
        code.removeAttribute("data-highlighted");
        delete code.result;
        delete code.second_best;
        return source;
    }}

    function highlightCode(code) {{
        if (!code) {{
            return;
        }}
        const source = resetCode(code);
        if (isPaxCode(code)) {{
            code.innerHTML = highlightPax(source);
            code.classList.add("hljs");
            return;
        }}
        if (window.hljs && typeof window.hljs.highlightElement === "function") {{
            window.hljs.highlightElement(code);
        }} else if (window.hljs && typeof window.hljs.highlightBlock === "function") {{
            window.hljs.highlightBlock(code);
        }}
        code.classList.add("hljs");
    }}

    function highlightPaxBlocks(root) {{
        (root || document)
            .querySelectorAll("code.language-pax, code.language-paxel, code.lang-pax, code.lang-paxel")
            .forEach(highlightCode);
    }}

    window.PaxHighlight = {{
        highlightCode,
        highlightPax,
        highlightPaxBlocks
    }};

    if (document.readyState === "loading") {{
        document.addEventListener("DOMContentLoaded", () => highlightPaxBlocks(document));
    }} else {{
        highlightPaxBlocks(document);
    }}
}})();
'''


if __name__ == "__main__":
    main()
