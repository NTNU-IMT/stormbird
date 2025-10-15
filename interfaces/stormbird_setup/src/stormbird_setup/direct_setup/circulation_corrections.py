

from ..base_model import StormbirdSetupBaseModel

from pydantic import model_serializer

from enum import Enum

class WindowSize(Enum):
    Five = "Five"
    Seven = "Seven"
    Nine = "Nine"

class GaussianSmoothingBuilder(StormbirdSetupBaseModel):
    smoothing_length_factor: float = 0.1
    number_of_end_points_to_interpolate: int = 0

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

class CirculationCorrectionBuilder(StormbirdSetupBaseModel):
    correction: CirculationSmoothingBuilder | None = None

    @classmethod
    def new_gaussian_smoothing(cls, smoothing_length_factor: float = 0.1, number_of_end_points_to_interpolate: int = 0):
        return cls(
            correction = CirculationSmoothingBuilder(
                smoothing_type = GaussianSmoothingBuilder(
                    smoothing_length_factor = smoothing_length_factor,
                    number_of_end_points_to_interpolate = number_of_end_points_to_interpolate
                )
            )
        )

    @model_serializer
    def ser_model(self):
        if self.correction is None:
            return "None"
        else:
            return {
                "Smoothing": self.correction.model_dump(exclude_none=True)
            }

