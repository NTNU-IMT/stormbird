from pydantic import Field

from ...base_model import StormbirdSetupBaseModel

from ..spatial_vector import SpatialVector

from .height_variation import HeightVariationModel
from .inflow_corrections import InflowCorrections

class WindEnvironment(StormbirdSetupBaseModel):
    height_variation_model: HeightVariationModel | None = HeightVariationModel()
    up_direction: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=1.0)
    )
    wind_rotation_axis: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=0.0, y=0.0, z=-1.0)
    )
    zero_direction_vector: SpatialVector = Field(
        default_factory=lambda: SpatialVector(x=1.0, y=0.0, z=0.0)
    )
    water_plane_height: float = 0.0
    inflow_corrections: InflowCorrections | None = None