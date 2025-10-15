

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings, DynamicSettings
from stormbird_setup.direct_setup.section_models import SectionModel

from stormbird_setup.direct_setup.lifting_line.solver import Linearized, SimpleIterative
from stormbird_setup.direct_setup.lifting_line.wake import QuasiSteadyWake, DynamicWake, SymmetryCondition

from stormbird_setup.direct_setup.circulation_corrections import CirculationCorrectionBuilder

from stormbird_setup.base_model import StormbirdSetupBaseModel

from enum import Enum

class SolverType(Enum):
    Linearized = "Linearized"
    SimpleIterative = "SimpleIterative"

class SingleWingSimulation(StormbirdSetupBaseModel):
    '''
    Simplified setup when the goal is to test only a single wing. A typical use case could be to 
    compare different simulation strategies and tune models against other more high-fidelity data 
    sources
    '''
    chord_length: float
    height: float
    section_model: SectionModel
    nr_sections: int = 32
    density: float = 1.225
    z_symmetry: bool = False
    dynamic: bool = False
    solver_type: SolverType = SolverType.Linearized
    smoothing_length: float = 0.0

    def get_line_force_model(self) -> LineForceModelBuilder:
        chord_vector = SpatialVector(x=self.chord_length)

        if self.z_symmetry:
            non_zero_circulation_at_ends = (True, False)
        else:
            non_zero_circulation_at_ends = (False, False)

        if self.smoothing_length > 0.0:
            circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(self.smoothing_length)
        else:
            circulation_correction = CirculationCorrectionBuilder()  # correction defaults to None

        wing_builder = WingBuilder(
            section_points = [
                SpatialVector(x=0.0, y=0.0, z=0.0),
                SpatialVector(x=0.0, y=0.0, z=self.height)
            ],
            chord_vectors = [
                chord_vector,
                chord_vector
            ],
            section_model = self.section_model,
            non_zero_circulation_at_ends = non_zero_circulation_at_ends
        )

        line_force_model = LineForceModelBuilder(
            nr_sections = self.nr_sections,
            density = self.density,
            circulation_correction = circulation_correction
        )

        line_force_model.add_wing_builder(wing_builder)

        return line_force_model
    
    def get_simulation_builder(self) -> SimulationBuilder:
        line_force_model = self.get_line_force_model()

        symmetry_condition = SymmetryCondition.Z if self.z_symmetry else SymmetryCondition.NoSymmetry

        if self.dynamic:
            match self.solver_type:
                case SolverType.SimpleIterative:
                    solver = SimpleIterative(
                        max_iterations_per_time_step = 40,
                        damping_factor = 0.05,
                    )
                case SolverType.Linearized:
                    solver = Linearized()

            wake = DynamicWake(
                symmetry_condition=symmetry_condition,
            )
            
            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = DynamicSettings(
                    solver = solver,
                    wake = wake
                )
            )
        else:
            match self.solver_type:
                case SolverType.SimpleIterative:
                    solver = SimpleIterative(
                        max_iterations_per_time_step = 1000,
                        damping_factor = 0.05,
                        start_with_linearized_solution = False
                    )
                case SolverType.Linearized:
                    solver = Linearized()

            wake = QuasiSteadyWake(
                symmetry_condition=symmetry_condition,
            )

            simulation_builder = SimulationBuilder(
                line_force_model = line_force_model,
                simulation_settings = QuasiSteadySettings(
                    solver = solver,
                    wake = wake
                )
            )

        return simulation_builder
    
