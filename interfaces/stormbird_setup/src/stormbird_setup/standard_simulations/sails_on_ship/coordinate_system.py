"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from enum import Enum

from ...spatial_vector import SpatialVector


class ForwardDirection(Enum):
    '''
    The direction of the bow along the x axis of the ship coordinate system. The z axis always
    points up, and the y axis completes a right-handed coordinate system, which means that y points
    to port when x points forward, and to starboard when x points backward.
    '''
    PositiveX = "PositiveX"
    NegativeX = "NegativeX"

    def vector(self) -> SpatialVector:
        '''Unit vector pointing towards the bow'''
        match self:
            case ForwardDirection.PositiveX:
                return SpatialVector(x=1.0)
            case ForwardDirection.NegativeX:
                return SpatialVector(x=-1.0)
            case _:
                raise ValueError("Unsupported forward direction:", self)


class Origin(Enum):
    '''
    The location of the origin of the ship coordinate system along the length of the ship. The
    origin is always at the water plane, i.e., z = 0 is the water plane.
    '''
    AftPerpendicular = "AftPerpendicular"
    Midship = "Midship"

    def ship_x_range(self, ship_length: float, forward_direction: ForwardDirection) -> tuple[float, float]:
        '''
        The smallest and largest x coordinate of the ship, approximated as the aft and forward
        perpendiculars.
        '''
        match self:
            case Origin.AftPerpendicular:
                aft, fore = 0.0, ship_length
            case Origin.Midship:
                aft, fore = -0.5 * ship_length, 0.5 * ship_length
            case _:
                raise ValueError("Unsupported origin:", self)

        if forward_direction == ForwardDirection.NegativeX:
            aft, fore = -aft, -fore

        return (min(aft, fore), max(aft, fore))
