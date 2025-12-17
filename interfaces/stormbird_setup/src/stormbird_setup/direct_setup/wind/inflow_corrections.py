
from ...base_model import StormbirdSetupBaseModel

class InflowCorrectionSingleSailSingleDirection(StormbirdSetupBaseModel):
    non_dimensional_span_distances: list[float]
    wake_factors_magnitude: list[float]
    angle_corrections: list[float]
    
class InflowCorrectionSingleSail(StormbirdSetupBaseModel):
    apparent_wind_directions: list[float]
    corrections: list[InflowCorrectionSingleSailSingleDirection]
    
class InflowCorrections(StormbirdSetupBaseModel):
    individual_corrections: list[InflowCorrectionSingleSail]