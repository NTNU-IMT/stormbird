"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math

from ..spatial_vector import SpatialVector
from ..base_model import StormbirdSetupBaseModel

class RefinementBuilder(StormbirdSetupBaseModel):
    '''
    Settings for a refinement zone in the mesh (snappyHexMesh)
    '''
    cells_per_representative_length: int
    """
    Target resolution inside the refinement zone. Since snappyHexMesh refines by splitting cells in
    two, the actual resolution will be the background resolution multiplied by a power of two, chosen
    so that it is at least as fine as this value.
    """
    padding: float = 1.0
    """
    Size of the refinement zone around each wing, which is a cylinder from the first to the last
    section point. The diameter of the cylinder is padding * mean chord length * the largest
    factor in the Gaussian projection function.
    """

class GridBuilder(StormbirdSetupBaseModel):
    '''
    Settings for the mesh of an OpenFOAM case. Unlike the uniform grid in Stormflow, the OpenFOAM
    mesh consists of a uniform background mesh (blockMesh) with optional refinement zones around the
    actuator lines and other geometries (snappyHexMesh).
    '''
    start_point: SpatialVector
    end_point: SpatialVector
    background_mesh_cells_per_representative_length: int
    """Resolution of the background mesh."""
    actuator_line_refinement: RefinementBuilder | None = None
    """Refinement zone around each wing in the actuator line model. None means no refinement."""
    geometry_cells_per_representative_length: int | None = None
    """
    Resolution at the surface of the other geometries in the case. How far the refinement extends
    away from the surfaces is controlled by the snappyHexMesh settings in the base folder. None
    means no refinement.
    """
    representative_length: float | None = None
    """
    An optional manually specified representative length. If not set, the average chord length of
    the wings in the actuator line model is used. Must be set if there is no actuator line model.
    """

    def background_mesh_shape(self, representative_length: float) -> tuple[int, int, int]:
        '''
        Number of cells along each axis of the background mesh. The numbers are rounded up, so that
        the cells are at least as fine as specified.
        '''
        cell_length = representative_length / self.background_mesh_cells_per_representative_length

        domain_lengths = (self.end_point - self.start_point).as_tuple()

        if any(length <= 0.0 for length in domain_lengths):
            raise ValueError("end_point must be larger than start_point along all axes")

        # Rounding before ceil avoids an extra cell due to floating point errors
        nx, ny, nz = (math.ceil(round(length / cell_length, 9)) for length in domain_lengths)

        return (nx, ny, nz)

    def refinement_level(self, cells_per_representative_length: int) -> int:
        '''
        The snappyHexMesh refinement level needed to get at least the given resolution, relative
        to the background mesh
        '''
        ratio = cells_per_representative_length / self.background_mesh_cells_per_representative_length

        if ratio <= 1.0:
            return 0

        # Rounding before ceil avoids an extra level due to floating point errors
        return math.ceil(round(math.log2(ratio), 9))
