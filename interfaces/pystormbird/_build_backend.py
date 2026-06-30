"""Custom build backend that checks for Rust before building.

This module wraps maturin's build backend to provide a clear error message
if Rust is not installed, rather than a cryptic build failure.
"""

import shutil
import sys

RUST_INSTALL_MESSAGE = """
================================================================================
ERROR: Rust toolchain not found!
================================================================================

pystormbird is a Rust extension that must be compiled from source.
To build this package, you need to install the Rust toolchain.

Installation instructions:
  - Visit https://rust-lang.org/ and follow the instructions
  
After installing Rust, restart your terminal and try again.
================================================================================
"""


def _check_rust_toolchain():
    """Check if Rust toolchain is available."""
    rustc = shutil.which("rustc")
    cargo = shutil.which("cargo")
    
    if rustc is None or cargo is None:
        print(RUST_INSTALL_MESSAGE, file=sys.stderr)
        sys.exit(1)


# Import and re-export all maturin build backend functions
from maturin import (
    build_wheel as _maturin_build_wheel,
    build_sdist as _maturin_build_sdist,
    build_editable as _maturin_build_editable,
    get_requires_for_build_wheel,
    get_requires_for_build_sdist,
    get_requires_for_build_editable,
)


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    """Build a wheel, checking for Rust first."""
    _check_rust_toolchain()
    return _maturin_build_wheel(wheel_directory, config_settings, metadata_directory)


def build_sdist(sdist_directory, config_settings=None):
    """Build a source distribution."""
    # No Rust check needed for sdist - it just packages the source
    return _maturin_build_sdist(sdist_directory, config_settings)


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    """Build an editable install, checking for Rust first."""
    _check_rust_toolchain()
    return _maturin_build_editable(wheel_directory, config_settings, metadata_directory)


# Re-export these as-is
__all__ = [
    "build_wheel",
    "build_sdist", 
    "build_editable",
    "get_requires_for_build_wheel",
    "get_requires_for_build_sdist",
    "get_requires_for_build_editable",
]
