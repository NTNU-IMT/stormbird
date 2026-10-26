
import numpy as np

from dataclasses import dataclass

from stormbird_setup.section_models import SectionModel, Foil
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.lifting_line import SymmetryCondition

from stormbird_setup.actuator_line import ActuatorLineBuilder, LiftingLineCorrectionBuilder

@dataclass(frozen=True, kw_only=True)
class StormbirdSettings():
    angle_of_attack_deg: float
    velocity: float = 1.0
    chord_length: float = 1.0
    span: float = 4.0
    use_ll_correction: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.velocity**2

    def get_line_force_model_builder(self):
        line_force_model_builder = LineForceModelBuilder(
            nr_sections=32,
            density=1.0
        )

        wing_builder = WingBuilder(
            section_points= [
                SpatialVector(z=0.0),
                SpatialVector(z=self.span)
            ],
            chord_vectors=[
                SpatialVector(x=self.chord_length),
                SpatialVector(x=self.chord_length)
            ],
            section_model = SectionModel(
                model = Foil(cl_zero_angle=0.5)
            )
        )

        line_force_model_builder.add_wing_builder(wing_builder)

        line_force_model_builder.local_wing_angles[0] = -np.radians(self.angle_of_attack_deg)

        return line_force_model_builder

    def get_actuator_line_setup_builder(self):
        line_force_model_builder = self.get_line_force_model_builder()

        al_builder = ActuatorLineBuilder(
            line_force_model = line_force_model_builder
        )

        if self.use_ll_correction:
            al_builder.lifting_line_correction = LiftingLineCorrectionBuilder(
                symmetry_condition = SymmetryCondition.Z
            )
        else:
            # The lifting line correction is used by default
            al_builder.lifting_line_correction = None

        return al_builder
