"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Reference data for simulations of sails on a ship, e.g., from experiments or other simulations, in a
standard format that the report can compare the simulations with.

The reference data is stored as JSON files with names ending in REFERENCE_FILE_SUFFIX, typically in
the same folder as the results. Each point in the reference data has a setup name and a test
condition, and is matched to the simulated cases with the same setup name and condition, rounded as
in the case names (see condition_key in naming.py). The solver is not part of the match, so a point
is compared with the results from all solvers.

The coefficients are given for each sail, in the same order as the wings in the actuator line setup,
and are defined as in the report: the thrust coefficient is the force along the forward direction of
the ship, and the side force coefficient is the force towards port, both divided by the dynamic
pressure of the undisturbed apparent wind at the mid-span of the sail and the projected area of the
sail. Coefficients that are not available are None.
"""

from pathlib import Path

from pydantic import field_validator, model_validator

from ...base_model import StormbirdSetupBaseModel

from .conditions import ShipTestCondition
from .naming import condition_key, check_setup_name

REFERENCE_FILE_SUFFIX = ".reference.json"


class ReferencePoint(StormbirdSetupBaseModel):
    '''The reference coefficients of each sail for one setup and test condition'''
    setup_name: str
    condition: ShipTestCondition
    thrust_coefficients: list[float | None] | None = None
    """Thrust coefficient of each sail, or None if not available"""
    side_force_coefficients: list[float | None] | None = None
    """Side force coefficient of each sail, or None if not available"""

    @field_validator("setup_name")
    @classmethod
    def check_name(cls, name: str) -> str:
        return check_setup_name(name)

    @model_validator(mode="after")
    def check_coefficients(self) -> "ReferencePoint":
        thrust = self.thrust_coefficients
        side_force = self.side_force_coefficients

        if thrust is None and side_force is None:
            raise ValueError("A reference point must have thrust or side force coefficients")

        if thrust is not None and side_force is not None and len(thrust) != len(side_force):
            raise ValueError(
                f"Got {len(thrust)} thrust coefficients and {len(side_force)} side force "
                "coefficients. Use None for the coefficients that are not available."
            )

        return self

    def key(self) -> str:
        '''The setup name and the rounded test condition, see condition_key'''
        return condition_key(self.condition, self.setup_name)

    def nr_sails(self) -> int:
        coefficients = self.thrust_coefficients or self.side_force_coefficients or []

        return len(coefficients)


class ReferenceData(StormbirdSetupBaseModel):
    '''A set of reference points from one source, e.g., an experiment'''
    label: str
    """Name of the source, shown in the report, e.g., "Wind tunnel" """
    points: list[ReferencePoint] = []

    @model_validator(mode="after")
    def check_unique_points(self) -> "ReferenceData":
        keys = [point.key() for point in self.points]

        duplicates = sorted(set(key for key in keys if keys.count(key) > 1))

        if duplicates:
            raise ValueError(
                f"The reference data '{self.label}' has several points for the same setup and "
                f"condition: {duplicates}"
            )

        return self

    def points_by_key(self) -> dict[str, ReferencePoint]:
        return {point.key(): point for point in self.points}


def reference_files_in_folder(folder: Path | str) -> list[Path]:
    '''The reference data files in a folder, i.e., the files ending in REFERENCE_FILE_SUFFIX'''
    return sorted(Path(folder).glob("*" + REFERENCE_FILE_SUFFIX))
