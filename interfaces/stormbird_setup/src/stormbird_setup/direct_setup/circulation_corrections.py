

from ..base_model import StormbirdSetupBaseModel

from pydantic import model_serializer, model_validator

from enum import Enum

class WindowSize(Enum):
    Five = "Five"
    Seven = "Seven"
    Nine = "Nine"

class GaussianSmoothingBuilder(StormbirdSetupBaseModel):
    smoothing_length_factor: float = 0.1

class CubicPolynomialSmoothingBuilder(StormbirdSetupBaseModel):
    window_size: WindowSize = WindowSize.Five

class CirculationSmoothingBuilder(StormbirdSetupBaseModel):
    smoothing_type: GaussianSmoothingBuilder = GaussianSmoothingBuilder()

    @model_serializer
    def ser_model(self):
        if isinstance(self.smoothing_type, GaussianSmoothingBuilder):
            return {
                "smoothing_type": {
                    "Gaussian":self.smoothing_type.model_dump()
                }
            }
        else:
            raise NotImplementedError("Only Gaussian smoothing is implemented")
            
    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data):
        if not isinstance(data, dict):
            return data
        
        if not data:
            return data
        
        # Check if smoothing_type needs to be unwrapped from Rust enum format
        if 'smoothing_type' in data:
            st = data['smoothing_type']
            if isinstance(st, dict):
                if 'Gaussian' in st:
                    return {'smoothing_type': GaussianSmoothingBuilder(**st['Gaussian'])}
                elif 'CubicPolynomial' in st:
                    return {'smoothing_type': CubicPolynomialSmoothingBuilder(**st['CubicPolynomial'])}
        
        return data
            
class PrescribedCirculationShape(StormbirdSetupBaseModel):
    inner_power: float = 2.0
    outer_power: float = 0.5
            
class PrescribedCirculation(StormbirdSetupBaseModel):
    shape: PrescribedCirculationShape = PrescribedCirculationShape()
    curve_fit_shape_parameters: bool = False

class CirculationCorrectionBuilder(StormbirdSetupBaseModel):
    correction: CirculationSmoothingBuilder | PrescribedCirculation | None = None

    @classmethod
    def new_gaussian_smoothing(
        cls, 
        smoothing_length_factor: float = 0.1,
    ):
        return cls(
            correction = CirculationSmoothingBuilder(
                smoothing_type = GaussianSmoothingBuilder(
                    smoothing_length_factor = smoothing_length_factor
                )
            )
        )
        
    @classmethod
    def new_prescribed_circulation(
        cls, 
        inner_power: float = 2.0, 
        outer_power: float = 0.5, 
        curve_fit_shape_parameters: bool = False
    ):
        return cls(
            correction = PrescribedCirculation(
                shape = PrescribedCirculationShape(
                    inner_power = inner_power,
                    outer_power = outer_power
                ),
                curve_fit_shape_parameters = curve_fit_shape_parameters
            )
        )
        
    @model_validator(mode='before')
    @classmethod
    def deserialize_from_rust_enum(cls, data):
        # Handle the "None" string case
        if data == "None":
            return {'correction': None}
        
        if not isinstance(data, dict):
            return data
        
        if not data:
            return data
        
        # Already in Python/Pydantic form
        if 'correction' in data:
            return data
        
        # Rust externally-tagged enum format
        if 'Prescribed' in data:
            return {'correction': PrescribedCirculation(**data['Prescribed'])}
        elif 'Smoothing' in data:
            # Use model_validate so it goes through CirculationSmoothingBuilder's validator
            return {'correction': CirculationSmoothingBuilder.model_validate(data['Smoothing'])}
        else:
            raise ValueError(f"Unknown circulation correction variant: {list(data.keys())}")
        
    @model_serializer
    def ser_model(self):
        if self.correction is None:
            return "None"
        elif isinstance(self.correction, PrescribedCirculation):
            return {
                "Prescribed": self.correction.model_dump(exclude_none=True)
            }
        elif isinstance(self.correction, CirculationSmoothingBuilder):
            return {
                "Smoothing": self.correction.ser_model()
            }
        else:
            raise ValueError("Invalid correction type")

