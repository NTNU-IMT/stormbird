"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from .power_model import PowerModel
from .logarithmic_model import LogarithmicModel

__all__ = [
    "PowerModel", "LogarithmicModel"
]
