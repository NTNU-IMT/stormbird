"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Minimal reader and writer for OpenFOAM dictionary files.

The parser only interprets the *structure* of a file. Sub-dictionaries become Python dicts, while
all other values are kept as raw strings, exactly as written in the file (minus comments). For
instance, the entry `internalField uniform (1 0 0);` is read as `{"internalField": "uniform (1 0 0)"}`.

Directives, such as `#include "functionObjects";`, are stored with the complete directive as the
key and `None` as the value, so that they are preserved in the right order.

When writing, the following Python types are supported as values:
- `dict`: written as a sub-dictionary
- `str`: written as is. Quotes must therefore be part of the string if they are needed.
- `bool`: written as `true` or `false`
- numbers: written using `str`
- `tuple`: written inline, e.g., `(1.0 2.0 3.0)`
- `list`: written as an OpenFOAM list, with one item per line
- `None`: only the key is written. Used for directives.

This is NOT a general purpose OpenFOAM parser. It is meant to handle the subset of the syntax that
is used in the base folder of this package, and it raises an error on syntax it does not understand.
"""

import numbers
from pathlib import Path

type FoamValue = str | bool | int | float | tuple[FoamValue, ...] | list[FoamValue] | FoamDict | None
type FoamDict = dict[str, FoamValue]

INDENT = "\t"

def read_foam_file(path: Path | str) -> FoamDict:
    '''
    Reads an OpenFOAM dictionary file and returns the content as a Python dict
    '''
    return parse_foam_string(Path(path).read_text())

def write_foam_file(path: Path | str, data: FoamDict) -> None:
    '''
    Writes a Python dict to an OpenFOAM dictionary file
    '''
    Path(path).write_text(format_foam_string(data))

def parse_foam_string(text: str) -> FoamDict:
    '''
    Parses the content of an OpenFOAM dictionary file
    '''
    return _Parser(text).parse_entries(top_level=True)

def format_foam_string(data: FoamDict) -> str:
    '''
    Formats a Python dict as the content of an OpenFOAM dictionary file
    '''
    return "".join(_format_entries(data, level=0))


class _Parser:
    def __init__(self, text: str):
        self.text = text
        self.pos = 0

    def error(self, message: str) -> ValueError:
        line_number = self.text.count("\n", 0, self.pos) + 1

        return ValueError(f"OpenFOAM parse error on line {line_number}: {message}")

    def peek(self, length: int = 1) -> str:
        return self.text[self.pos:self.pos + length]

    def at_end(self) -> bool:
        return self.pos >= len(self.text)

    def skip_comment(self) -> bool:
        '''
        Skips a comment at the current position, if there is one. Returns True if a comment was
        skipped.
        '''
        if self.peek(2) == "//":
            end = self.text.find("\n", self.pos)
            self.pos = len(self.text) if end == -1 else end

            return True

        if self.peek(2) == "/*":
            end = self.text.find("*/", self.pos + 2)

            if end == -1:
                raise self.error("unterminated block comment")

            self.pos = end + 2

            return True

        return False

    def skip_whitespace_and_comments(self) -> None:
        while not self.at_end():
            if self.peek().isspace():
                self.pos += 1
            elif not self.skip_comment():
                return

    def read_until(self, terminator: str, error_message: str) -> str:
        '''
        Reads from the current position up to and including the terminator
        '''
        end = self.text.find(terminator, self.pos)

        if end == -1:
            raise self.error(error_message)

        start = self.pos
        self.pos = end + len(terminator)

        return self.text[start:self.pos]

    def read_quoted_string(self) -> str:
        start = self.pos
        self.pos += 1

        while not self.at_end():
            character = self.peek()

            if character == "\\":
                self.pos += 2
            elif character == '"':
                self.pos += 1

                return self.text[start:self.pos]
            else:
                self.pos += 1

        raise self.error("unterminated string")

    def read_word(self) -> str:
        '''
        Reads a keyword or directive argument. Parentheses are allowed as part of the word, to
        support keywords such as `div(phi,U)`.
        '''
        if self.peek() == '"':
            return self.read_quoted_string()

        start = self.pos
        depth = 0

        while not self.at_end():
            character = self.peek()

            if character.isspace() or character in ";{}":
                break

            if character == "(":
                depth += 1
            elif character == ")":
                if depth == 0:
                    break

                depth -= 1

            self.pos += 1

        if self.pos == start:
            raise self.error(f"expected a keyword, found '{self.peek()}'")

        return self.text[start:self.pos]

    def read_directive(self) -> str:
        '''
        Reads a directive, such as `#include "file";`, and returns the complete text of it
        '''
        start = self.pos
        self.pos += 1
        self.read_word()

        while self.peek() in (" ", "\t"):
            self.pos += 1

        if self.peek() not in ("\n", ";", ""):
            self.read_word()

        if self.peek() == ";":
            self.pos += 1

        return self.text[start:self.pos]

    def read_raw_value(self) -> str:
        '''
        Reads everything up to the next semicolon that is not nested inside parentheses, braces,
        strings or code blocks. Comments are removed.
        '''
        chunks: list[str] = []
        chunk_start = self.pos
        depth = 0

        while True:
            if self.at_end():
                raise self.error("missing ';' at end of entry")

            if self.peek(2) in ("//", "/*"):
                chunks.append(self.text[chunk_start:self.pos])
                self.skip_comment()
                chunks.append(" ")
                chunk_start = self.pos

                continue

            character = self.peek()

            if self.peek(2) == "#{":
                self.read_until("#}", "unterminated code block")
            elif character == '"':
                self.read_quoted_string()
            elif character in "({":
                depth += 1
                self.pos += 1
            elif character in ")}":
                depth -= 1

                if depth < 0:
                    raise self.error(f"unexpected '{character}'. Is a ';' missing?")

                self.pos += 1
            elif character == ";" and depth == 0:
                chunks.append(self.text[chunk_start:self.pos])
                self.pos += 1

                return "".join(chunks).strip()
            else:
                self.pos += 1

    def parse_entries(self, top_level: bool) -> FoamDict:
        '''
        Parses entries until the end of the current dictionary
        '''
        entries: FoamDict = {}

        while True:
            self.skip_whitespace_and_comments()

            if self.at_end():
                if not top_level:
                    raise self.error("missing '}' at end of dictionary")

                return entries

            if self.peek() == "}":
                if top_level:
                    raise self.error("unexpected '}'")

                self.pos += 1

                return entries

            if self.peek() == "#":
                key = self.read_directive()
                value: FoamValue = None
            else:
                key = self.read_word()
                self.skip_whitespace_and_comments()

                if self.peek() == "{":
                    self.pos += 1
                    value = self.parse_entries(top_level=False)
                else:
                    value = self.read_raw_value()

            if key in entries:
                raise self.error(f"duplicate keyword '{key}'")

            entries[key] = value


def _format_scalar(value: FoamValue) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"

    if isinstance(value, str):
        return value

    if isinstance(value, numbers.Number):
        return str(value)

    if isinstance(value, tuple):
        return "(" + " ".join(_format_scalar(item) for item in value) + ")"

    raise TypeError(f"unsupported value of type {type(value).__name__} in OpenFOAM data: {value!r}")

def _format_list(items: list[FoamValue], level: int) -> list[str]:
    indent = INDENT * level
    lines = [f"{indent}(\n"]

    for item in items:
        if isinstance(item, dict):
            lines.append(f"{indent}{INDENT}{{\n")
            lines.extend(_format_entries(item, level + 2))
            lines.append(f"{indent}{INDENT}}}\n")
        elif isinstance(item, list):
            lines.extend(_format_list(item, level + 1))
        else:
            lines.append(f"{indent}{INDENT}{_format_scalar(item)}\n")

    lines.append(f"{indent})")

    return lines

def _format_entries(data: FoamDict, level: int) -> list[str]:
    indent = INDENT * level
    lines: list[str] = []

    for key, value in data.items():
        if value is None:
            lines.append(f"{indent}{key}\n")
        elif isinstance(value, dict):
            lines.append(f"{indent}{key}\n{indent}{{\n")
            lines.extend(_format_entries(value, level + 1))
            lines.append(f"{indent}}}\n\n")
        elif isinstance(value, list):
            lines.append(f"{indent}{key}\n")
            lines.extend(_format_list(value, level))
            lines.append(";\n")
        else:
            text = _format_scalar(value)

            lines.append(f"{indent}{key} {text};\n" if text else f"{indent}{key};\n")

    return lines
