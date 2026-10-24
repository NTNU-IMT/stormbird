"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import csv
import json
from pathlib import Path
from typing import Any

from pydantic import Field

from ..base_model import StormbirdSetupBaseModel

class ActuatorLineResults(StormbirdSetupBaseModel):
    '''
    The results from the actuator line model, as written to the postProcessing folder of a case
    '''
    forces: dict[str, list[float]]
    """The columns in stormbird_forces.csv, with the column names as keys."""
    full_results: dict[int, dict[str, Any]] = Field(default_factory=dict)
    """The files in stormbird_full_results, with the iteration number as key."""

    @classmethod
    def from_case(cls, path: Path | str) -> "ActuatorLineResults":
        post_processing_path = Path(path) / "postProcessing"

        with open(post_processing_path / "stormbird_forces.csv", newline="") as file:
            rows = list(csv.reader(file))

        header, data = rows[0], rows[1:]

        forces = {
            name: [float(row[index]) for row in data] for index, name in enumerate(header)
        }

        full_results: dict[int, dict[str, Any]] = {}

        for file_path in (post_processing_path / "stormbird_full_results").glob("full_results_*.json"):
            iteration = int(file_path.stem.removeprefix("full_results_"))

            full_results[iteration] = json.loads(file_path.read_text())

        return cls(forces=forces, full_results=dict(sorted(full_results.items())))
