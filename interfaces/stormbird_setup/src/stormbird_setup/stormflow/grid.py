
from ..spatial_vector import SpatialVector
from ..base_model import StormbirdSetupBaseModel

class GridBuilder(StormbirdSetupBaseModel):
    start_point: SpatialVector
    end_point: SpatialVector
    cells_per_representative_length: tuple[int, int, int]
    representative_length: float | None = None
    max_cells_per_axis_after_coarsening: int | None = None
    