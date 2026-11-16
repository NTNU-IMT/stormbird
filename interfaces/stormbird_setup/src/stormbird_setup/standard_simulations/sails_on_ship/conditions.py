"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ...base_model import StormbirdSetupBaseModel


class WindProfile(Enum):
    '''
    How the true wind velocity varies with the height above the water plane
    '''
    PowerLaw = "PowerLaw"
    """
    Atmospheric boundary layer, modelled with a power law with the default settings of PowerModel,
    where the true wind velocity in the condition is the velocity at 10 m.
    """
    Constant = "Constant"
    """The same true wind velocity at all heights, i.e., no atmospheric boundary layer."""


class ShipTestCondition(StormbirdSetupBaseModel):
    '''
    One test condition for sails on a ship: the wind, the ship velocity and, optionally, a fixed
    setting of the sails.

    The sign of the angle of attack and the internal state is set automatically for each wing, so
    that positive values give lift with a positive component along the forward direction of the
    ship, i.e., thrust. The same values can therefore be used for wind from both sides.

    (Named ShipTestCondition rather than TestCondition, so that pytest does not try to collect it
    as a test class.)
    '''
    true_wind_direction: float
    """
    Direction the true wind is coming from, in radians, measured clockwise seen from above, with
    zero as a head wind. Positive values are wind from starboard.
    """
    true_wind_velocity: float
    """
    True wind velocity at the reference height of the wind profile, which is 10 m, or at all heights
    with a constant wind profile.
    """
    ship_velocity: float
    wind_profile: WindProfile = WindProfile.PowerLaw
    angle_of_attack: float | None = None
    """
    Fixed geometric angle of attack, in radians, relative to the undisturbed apparent wind at the
    mid-span of each wing. The wings are then kept at a fixed angle, and any controller in the
    actuator line setup is removed. None means that the actuator line setup is used as given, e.g.,
    with its controller.
    """
    internal_state: float | None = None
    """
    Fixed internal state of each wing, e.g., the suction coefficient, with the sign set
    automatically, as for the angle of attack. None means that the internal states in the actuator
    line setup are used as given.
    """
