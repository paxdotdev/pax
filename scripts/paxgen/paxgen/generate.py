#!/usr/bin/env python

import io
import re
import subprocess
import sys
from pathlib import Path
from functools import wraps, cache
from typing import Callable, TextIO

from rich import print
from rich.panel import Panel
from rich.console import Console
from rich.markup import escape
import tiktoken

console = Console()

ENCODING = tiktoken.get_encoding("o200k_base")
DEFAULT_DESCRIPTION = "A blue square rotating against a red background."


@cache
def pax_root() -> Path:
    """Returns the root of the pax-engine/pax repository."""
    default = "~/code/pax"
    answer = input(f"Path to your pax engine repository? [{default}] ") or default
    path = Path(answer).expanduser().absolute()
    print(f"[yellow]Using path for pax engine:[/yellow] {str(path)}")
    assert path.exists()
    return path


@cache
def pax_docs() -> Path:
    """Returns the root of the pax-engine/pax repository."""
    default = "~/code/docs"
    answer = input(f"Path to your pax docs repository? [{default}] ") or default
    path = Path(answer).expanduser().absolute()
    print(f"[yellow]Using path for pax docs:[/yellow] {str(path)}")
    assert path.exists()
    return path


# =====================================================================
#  Utilities
# =====================================================================

def copy_to_clipboard(text: str):
    process = subprocess.Popen(["pbcopy"], stdin=subprocess.PIPE)
    process.communicate(input=text.encode("utf-8"))


def increment_markdown(n: int) -> Callable[[Callable[..., str]], Callable[..., str]]:
    def decorator(fn: Callable[..., str]) -> Callable[..., str]:
        @wraps(fn)
        def _fn(*args, **kwargs) -> str:
            return add_extra_hash(fn(*args, **kwargs), n=n)
        return _fn
    return decorator


def add_extra_hash(text: str, n: int) -> str:
    """Find sequences of '#' and add an extra '#' to each sequence."""
    def replace_match(match: re.Match) -> str:
        hash_sequence = match.group(1)
        if n >= 0:
            return hash_sequence + '#' * n
        else:
            return hash_sequence[:n]
    return re.sub(r'(#+)', replace_match, text)


def count_tokens(text: str) -> int:
    return len(ENCODING.encode(text))


def print_token_count(fn: Callable[..., str]) -> Callable[..., str]:
    """Prints token count of the output of the inner function."""
    @wraps(fn)
    def _fn(*args, **kwargs) -> str:
        output = fn(*args, **kwargs)
        count = count_tokens(output)
        print(f"[white]Function `{fn.__name__}()` outputted {count} tokens[/white]")
        return output
    return _fn


def strip_output(fn: Callable[..., str]) -> Callable[..., str]:
    """Strips output text of the inner function."""
    @wraps(fn)
    def _fn(*args, **kwargs) -> str:
        return fn(*args, **kwargs).strip()
    return _fn


def text_between_substrings(
    text: str | TextIO,
    start: str | None = None,
    stop: str | None = None,
    include_stop: bool = False,
):
    """Get the substring between the two input strings."""
    if isinstance(text, io.IOBase):
        text = text.read()

    if start is None:
        start = "^"
    else:
        start = re.escape(start)

    if stop is None:
        stop = "$"
        include_stop = True
    else:
        stop = re.escape(stop)
        
    if include_stop:
        pattern = start + '(.*?)' + stop
    else:
        pattern = start + '(.*?)' + stop

    if match := re.search(pattern, text, re.DOTALL):
        if include_stop:
            return match.group(0)
        else:
            start_index = match.start(1)
            return text[match.start():start_index] + match.group(1)
    else:
        raise RuntimeError(f"{start=} {stop=}")


def text_between_anchors(text: str | TextIO, anchor: str):
    """Get the substring between two invisible anchors in Markdown.
    
    Never include the text of the anchors, and strip newlines around
    the results. This works when the text is formatted in Markdown and
    the sections we want to extract are delimited with invisible anchors:
    
    ```markdown
    This is some documentation we are leaving out.

    [//]: # (<anchor hash:start>)

    This is text we'd like to keep!

    [//]: # (<anchor hash:stop>)

    And here we are leaving this out again.
    ```

    This should extract to "This is text we'd like to keep!". Note that
    each anchor should have a preceding and a following newline, just
    to be safe.
    """
    if isinstance(text, io.IOBase):
        text = text.read()

    start_regex = rf"\[//\]: # \({anchor}:start\)"
    stop_regex = rf"\[//\]: # \({anchor}:stop\)"
    full_regex = start_regex + "(.*?)" + stop_regex

    if match := re.search(full_regex, text, re.DOTALL):
        return match.group(1).strip()
    
    raise RuntimeError(f"Could not extract text with {anchor=}")


# =====================================================================
#  Prompt fragments
# =====================================================================

@print_token_count
@strip_output
def what_is_pax() -> str:
    """Modified from the header of `src/introduction/intro-pax.md`."""
    return f"""Pax is a framework to build user interfaces.

Write application logic in Rust and declare your user interface in Pax's user interface description language, called PAXEL.

Pax compiles into native desktop/mobile apps, WebAssembly-driven sites, and embeddable universal UI components.
"""


# When using anchors, this script finds the relevant portions of the Pax
# docs and examples by regex-matching lines in markdown between lines of
# the form:
#
# [//]: # (62e3c3e1ceb61335e83d631afbbb5080:start)
#
#     ...
# [//]: # (62e3c3e1ceb61335e83d631afbbb5080:stop)
#
#
# The nice thing is that those anchors are invisible in the rendered
# Markdown itself. Otherwise, we have to resort to matching substrings
# in the text itself, which is more brittle. We leave the codepaths
# with USE_ANCHORS=False below, for backwards compatibility.
# 
# See also: `text_between_anchors()` in this file.
USE_ANCHORS = True


@print_token_count
@strip_output
def get_paxel_grammar_definition() -> str:
    with open(pax_root() / "pax-lang/src/pax.pest", mode="r") as f:
        return f.read()


@print_token_count
@strip_output
def get_paxel_explainer() -> str:
    with open(pax_docs() / "pages/reference/paxel.md", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "62e3c3e1ceb61335e83d631afbbb5080")
        else:
            return text_between_substrings(
                text=f.read(),
                start="PAXEL is compiled by transpiling through Rust",
                stop="With a reasonable and finite amount of work",
                include_stop=False,
            )


@print_token_count
@strip_output
def get_docs_components() -> str:
    with open(pax_docs() / "pages/key-concepts/components.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "359630593ac547b4459c5e7bcd7a65c3")
        else:
            return text_between_substrings(
                text=f.read(),
                start="The atomic unit of Pax is a",
                stop="inside a template in that file.",
                include_stop=True,
            )


@print_token_count
@strip_output
@increment_markdown(1)
def get_docs_templates() -> str:
    with open(pax_docs() / "pages/key-concepts/templates.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "8a4bfc8b47c07012ebbf0328529e8417")
        else:
            return text_between_substrings(
                text=f.read(),
                start="A component's _template_ describes",
            )


@print_token_count
@strip_output
@increment_markdown(1)
def get_docs_properties() -> str:
    with open(pax_docs() / "pages/key-concepts/settings.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "1bdb47551ded2905735ab34924507ff0")
        else:
            return text_between_substrings(
                text=f.read(),
                start="## Properties",
                stop="enqueued transitions are completed.",
                include_stop=True,
            )


@print_token_count
@strip_output
def get_docs_expressions() -> str:
    with open(pax_docs() / "pages/key-concepts/expressions.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "cbe9f37cef36c8fa421f6ca500ac24bf")
        else:
            return text_between_substrings(
                text=f.read(),
                start="```jsx",
            )


@print_token_count
@strip_output
def get_docs_coordinates() -> str:
    with open(pax_docs() / "pages/reference/layout.md", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "9732c66d1335615593249a6e623479a8")
        else:
            return text_between_substrings(
                text=f.read(),
                start="Pax's coordinate system",
                stop="positioning and resizing logic.",
                include_stop=True,
            )


@print_token_count
@strip_output
@increment_markdown(1)
def get_docs_event_handlers() -> str:
    with open(pax_docs() / "pages/key-concepts/handlers.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "8131d6966a7f9a63c435deaf5b79b664")
        else:
            return text_between_substrings(
                text=f.read(),
                start="```rust",
                stop="- Wheel (@wheel)",
                include_stop=True,
            )


@print_token_count
@strip_output
@increment_markdown(1)
def get_docs_primitives() -> str:
    with open(pax_docs() / "pages/key-concepts/primitives.mdx", mode="r") as f:
        if USE_ANCHORS:
            return text_between_anchors(f, "fb144467945d10ab68f81eda3af4bde3")
        else:
            return text_between_substrings(
                text=f.read(),
                start="Primitives are a special case of `component`",
            )


@print_token_count
@strip_output
def get_examples_as_xml() -> str:
    """Find all examples and embed each relevant file as XML."""
    xml_tags: list[str] = []

    for example in (pax_root() / "examples/src").iterdir():
        if not example.is_dir():
            # Skip non-directories
            continue

        if not (source := example / "src").exists() or not source.is_dir():
            # Skip if we cannot find an "src/" subdirectory
            continue
        
        xml_tags.append(f"<example name=\"{example.name}\">")
        
        for file in source.iterdir():
            if not file.is_file():
                continue

            file_contents = file.read_text(encoding="utf-8")
            xml_tags.append (f"  <file name=\"{file.name}\">")
            xml_tags.append (f"    <contents>")
            xml_tags.append (file_contents)
            xml_tags.append (f"    </contents>")
            xml_tags.append (f"  </file>")
    
        xml_tags.append("</example>")
    
    return "\n".join(xml_tags)


@print_token_count
@strip_output
def get_examples_as_md() -> str:
    """Find all examples and embed each relevant file as Markdown."""
    fragments: list[str] = []

    for example in (pax_root() / "examples/src").iterdir():
        if not example.is_dir():
            # Skip non-directories
            continue

        if not (source := example / "src").exists() or not source.is_dir():
            # Skip if we cannot find an "src/" subdirectory
            continue
        
        fragments.append(f"## Example: `{example.name}`\n")
        
        for file in source.iterdir():
            if not file.is_file():
                continue
            
            fragments.append (f"### File: `{file.name}`\n")
            if file.suffix == ".rs":
                fragments.append (f"```rust")
            elif file.suffix == ".pax":
                fragments.append (f"```paxel")
            else:
                fragments.append (f"```")
            file_contents = file.read_text(encoding="utf-8")
            fragments.append (file_contents)
            fragments.append (f"```\n")
    
    return "\n".join(fragments)


@print_token_count
@strip_output
def get_instructions_preamble(instructions: str) -> str:
    return f"""Please generate content for two files: one named `lib.rs` (containing Rust code) and one named `lib.pax` (containing PAXEL code) to help the user create a user interface matching the following description:

{instructions.strip()}

Please pay special attention to the user instructions above, and please rely heavily on the documentation and examples provided in this prompt to help you write valid Pax (Rust and PAXEL) code. Please provide the contents for each file using the XML format illustrated in the `# Examples` section.
"""


# =====================================================================
#  Full prompt
# =====================================================================

@strip_output
def make_prompt_v2(description: str = "<NOT PROVIDED>") -> str:
    return f"""
# Goal

You are a Large Language Model and you are prompted to help the user generate code in two languages: Rust, which should be in your training data, and PAXEL, for which relevant language information and documentation is available below.

At the end of this prompt, instructions will be given to you under the header `# Instructions`. Until then, please pay close attention to the rest of this prompt as it contains critical information.

# Pax & PAXEL documentation

## What is Pax

{what_is_pax()}

## What is PAXEL

PAXEL is Pax's expression language. {get_paxel_explainer()}

## PAXEL grammar

Here is the grammar of the PAXEL language inlined in the PEST file format:

```
{get_paxel_grammar_definition()}
```

## Components

{get_docs_components()}

## Templates

{get_docs_templates()}

## Properties and settings

`Properties` and `Settings` are two sides of the same idea, so they share a chapter in this book. Recall that the atomic unit of Pax is the component. Components pass data to each other through `properties` and `settings`.

{get_docs_properties()}

## Expressions

{get_docs_expressions()}

## Coordinate System & Transforms

{get_docs_coordinates()}

## Event handlers

{get_docs_event_handlers()}

## Primitives

{get_docs_primitives()}

# Examples

In order to greatly improve your ability to generate valid and high-quality Pax code, valid examples of user interfaces built in Pax are included below. Feel free to quote any relevant sections if it helps you write better code or reason more precisely.

{get_examples_as_md()}

# Instructions

{get_instructions_preamble(description)}
"""


# =====================================================================
#  CLI entrypoint
# =====================================================================

def main():
    try:
        _, description = sys.argv
    except ValueError:
        description = input("Please write a UI description: ")
    
    if not description:
        description = description or DEFAULT_DESCRIPTION
        print(f"[yellow]Using default prompt:[/yellow] {description}")

    pax_root()  # asks for path and caches answer
    pax_docs()  # asks for path and caches answer
    p = make_prompt_v2(description)
    n = count_tokens(p)

    input(f"Press Enter to display full prompt ({n} tokens-long)")
    with console.pager():
        console.print(Panel.fit(escape(p)))
    print(f"[white]Token count: prompt has {n} tokens[/white]")

    copy_to_clipboard(p)
    print("[blue]Copied prompt to clipboard![/blue]")


if __name__ == "__main__":
    main()