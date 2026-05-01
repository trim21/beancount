from typing import Any

from beancount.core import data

__version__: str

class ParserSpan:
    source_id: str
    filename: str
    start: int
    end: int
    line: int
    column: int
    start_line: int
    end_line: int
    excerpt: str

class ParserLabelSpan:
    label: str
    span: ParserSpan

class ParserError:
    source: data.Meta
    message: str
    entry: data.Directive | None
    filename: str
    line: int
    column: int
    span_start: int | None
    span_end: int | None
    source_id: str
    kind: str
    reason: str
    span: ParserSpan | None
    contexts: list[ParserLabelSpan]
    related: list[ParserLabelSpan]
    annotation: str | None

__all__ = [
    "__version__",
    "ParserError",
    "ParserSpan",
    "ParserLabelSpan",
    "build_options_map",
    "check_file_rust",
    "load_file",
    "load_file_and_book",
    "load_string_and_book",
    "parse_string",
]

def load_file(
    filename: str,
) -> tuple[data.Directives, list[ParserError], dict[str, Any]]: ...
def parse_string(
    content: str, filename: str | None = ...
) -> tuple[data.Directives, list[ParserError], dict[str, Any]]: ...
def check_file_rust(filename: str) -> tuple[int, int, int, int]: ...
def load_file_and_book(
    filename: str,
) -> tuple[data.Directives, list[Any], dict[str, Any]]: ...
def load_string_and_book(
    content: str, filename: str = ...
) -> tuple[data.Directives, list[Any], dict[str, Any]]: ...
def build_options_map(filename: str | None = ...) -> dict[str, Any]: ...
