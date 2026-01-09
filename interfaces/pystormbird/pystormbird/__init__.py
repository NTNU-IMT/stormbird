"""Pystormbird - Python interface to Stormbird."""

import sys
from pystormbird import _native

# Register native submodules in sys.modules so they're importable
sys.modules['pystormbird._native.lifting_line'] = _native.lifting_line
sys.modules['pystormbird._native.section_models'] = _native.section_models
sys.modules['pystormbird._native.line_force_model'] = _native.line_force_model
sys.modules['pystormbird._native.wind'] = _native.wind
sys.modules['pystormbird._native.smoothing'] = _native.smoothing

# Re-export top-level classes
from pystormbird._native import SimulationResult, SectionalForcesInput

# Make submodules available
from pystormbird import lifting_line
from pystormbird import section_models
from pystormbird import line_force_model
from pystormbird import wind
from pystormbird import smoothing

__all__ = [
    "SimulationResult",
    "SectionalForcesInput",
    "lifting_line",
    "section_models",
    "line_force_model",
    "wind",
    "smoothing",
]