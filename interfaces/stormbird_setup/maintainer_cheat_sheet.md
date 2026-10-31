# Maintainer cheat-sheet

Some basic instructions for the maintainer to remember how to do different things, to avoid having to Google the same thing over and over again...

## Publishing to PyPI (maintainer cheat-sheet)

> This section is a reminder for the main developer. Only the package owner has upload rights to PyPI.

This is a pure Python package, so both a source distribution and a wheel are uploaded.

1. Bump `version` in `pyproject.toml` and update `CHANGELOG.md`.
2. Remove old build artifacts and build:
   ```bash
   rm -rf dist build
   pip install build twine
   python -m build
   ```
3. Upload with an API token from PyPI (username `__token__`, password = the token):
   ```bash
   twine upload dist/*
   ```