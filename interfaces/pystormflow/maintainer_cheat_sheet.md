# Maintainer cheat-sheet

Some basic instructions for the maintainer to remember how to do different things, to avoid having to Google the same thing over and over again...

## Publishing to PyPI (maintainer cheat-sheet)

> This section is a reminder for the main developer. Only the package owner has upload rights to PyPI.

The package is published as a **source distribution only** (no pre-built wheels), so users compile it locally with their own Rust toolchain. Do not upload wheels from `maturin build`.

1. Bump `version` in `Cargo.toml` (the Python version is read from there) and update `CHANGELOG.md`.
2. Remove old build artifacts and build the source distribution:
   ```bash
   rm -rf target/wheels
   maturin sdist
   ```
3. Optionally check that the sdist installs: `pip install target/wheels/pystormflow-*.tar.gz`
4. Upload with an API token from PyPI (username `__token__`, password = the token):
   ```bash
   pip install twine
   twine upload target/wheels/pystormflow-*.tar.gz
   ```