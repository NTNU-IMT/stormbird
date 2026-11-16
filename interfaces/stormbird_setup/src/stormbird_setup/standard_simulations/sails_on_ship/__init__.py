"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

Standard setup of actuator line simulations of sails on a ship.
"""

from .coordinate_system import ForwardDirection, Origin
from .conditions import ShipTestCondition, WindProfile
from .simulation_settings import (
    CfdSolver, ShipDomain, OpenFoamShipSettings, StormflowShipSettings, StormflowRunSettings,
    AIR_KINEMATIC_VISCOSITY
)
from .naming import case_name, parse_case_name, check_setup_name, ParsedCaseName
from .sails_on_ship import SailsOnShip
from .report import sails_on_ship_report, read_report_cases, make_overview_figure, make_detailed_figure

__all__ = [
    "sails_on_ship_report",
    "read_report_cases",
    "make_overview_figure",
    "make_detailed_figure",
    "case_name",
    "parse_case_name",
    "check_setup_name",
    "ParsedCaseName",
    "StormflowRunSettings",
    "ForwardDirection",
    "Origin",
    "ShipTestCondition",
    "WindProfile",
    "CfdSolver",
    "ShipDomain",
    "OpenFoamShipSettings",
    "StormflowShipSettings",
    "AIR_KINEMATIC_VISCOSITY",
    "SailsOnShip",
]
