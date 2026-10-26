'''
Functions for setting up and running simulations of the three-dimensional suction sail model from
the paper, shared by the scripts in this example.
'''

from dataclasses import dataclass, field

import numpy as np

from stormbird_setup.default_sails import DefaultSail, SailDefinition, SolverType
from stormbird_setup.section_models import (
    InternalStateConversion,
    InternalStateConversionType,
)
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.line_force_model import LineForceModelBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings
from stormbird_setup.lifting_line.wake import QuasiSteadyWakeSettings, SymmetryCondition

from pystormbird.lifting_line import Simulation

# Internal state conversion where the internal state is the suction coefficient, used directly by
# the foil model. Used when the suction coefficient is given, rather than the power.
SUCTION_COEFFICIENT_DIRECT = InternalStateConversion(
    conversion_type = InternalStateConversionType.SuctionCoefficientDirect
)

@dataclass
class SimulationResult:
    cl: float
    cd: float
    cp: float
    section_internal_states: np.ndarray
    section_angles_of_attack: np.ndarray
    section_velocities: np.ndarray
    section_is_virtual: np.ndarray

    def wing_suction_coefficient(self, velocity) -> float:
        '''
        The suction coefficient for the whole wing, Q / (U A), from the suction coefficient on each
        line element, which all have the same area. The virtual line elements are not part of the
        sail, and are therefore not included.
        '''
        real = ~self.section_is_virtual

        return np.mean(
            np.abs(self.section_internal_states[real]) * self.section_velocities[real]
        ) / velocity

@dataclass(kw_only=True)
class SuctionSailModel:
    '''
    Lifting line model of the wind tunnel model in the paper, with one end against the wind tunnel
    wall. The section model and the simulation settings are taken from the sail definition, which is
    the default suction sail model in stormbird_setup. The version for the iterative solver is used
    by default, as the model was tuned with it.
    '''
    chord_length: float = 1.0
    height: float = 4.0
    nr_sections: int = 32
    density: float = 1.225
    sail_definition: SailDefinition = field(
        default_factory = lambda: DefaultSail.SuctionSailCharrier1985.definition(SolverType.Iterative)
    )
    # Replaces the internal state conversion in the sail definition, if set. Without it, the
    # internal state is the input power to the fan.
    internal_state_conversion: InternalStateConversion | None = None

    @property
    def area(self) -> float:
        return self.chord_length * self.height

    def get_line_force_model(self) -> LineForceModelBuilder:
        # The wing starts at the symmetry plane, which represents the wind tunnel wall. The
        # circulation is not zero at the bottom of the wing, as the wing continues as a mirror image
        # below the plane.
        wing_builder = self.sail_definition.wing_builder(
            position = SpatialVector(x=0.0, y=0.0, z=0.0),
            chord_length = self.chord_length,
            height = self.height,
            base_on_symmetry_plane = True
        )

        if self.internal_state_conversion is not None:
            wing_builder.section_model.model.internal_state_conversion = self.internal_state_conversion

        line_force_model = LineForceModelBuilder(
            nr_sections = self.nr_sections,
            density = self.density,
            circulation_correction = self.sail_definition.circulation_correction
        )

        line_force_model.add_wing_builder(wing_builder)

        return line_force_model

    def get_simulation(self) -> Simulation:
        simulation_settings = QuasiSteadySettings(
            solver = self.sail_definition.solver,
            wake = QuasiSteadyWakeSettings(
                symmetry_condition = SymmetryCondition.Z
            )
        )

        simulation_builder = SimulationBuilder(
            line_force_model = self.get_line_force_model(),
            simulation_settings = simulation_settings
        )

        return Simulation(simulation_builder.to_json_string())

    def run_case(
        self,
        *,
        velocity: float, 
        angle_of_attack_deg: float, 
        internal_state: float
    ) -> SimulationResult:
        simulation = self.get_simulation()
        
        freestream_velocity = [
            [velocity, 0.0, 0.0] for _ in simulation.get_freestream_velocity_points()
        ]
    
        simulation.set_local_wing_angles([-np.radians(angle_of_attack_deg)])
        simulation.set_internal_state([internal_state])
    
        result = simulation.do_step(
            time = 0.0,
            time_step = 1.0,
            freestream_velocity = freestream_velocity
        )
    
        force_factor = 0.5 * self.density * self.area * velocity**2
    
        force = result.integrated_forces[0].total

    
        return SimulationResult(
            cl = force[1] / force_factor,
            cd = force[0] / force_factor,
            cp = result.input_power[0] / (force_factor * velocity),
            section_internal_states = np.array(result.force_input.section_models_internal_state),
            section_angles_of_attack = np.array(result.force_input.angles_of_attack),
            section_velocities = np.linalg.norm(np.array(result.force_input.velocity), axis=1),
            section_is_virtual = np.array(result.ctrl_points)[:, 2] > self.height,
        )

    def angle_sweep(
        self,
        *,
        velocity: float,
        angles_of_attack_deg: np.ndarray,
        internal_state: float
    ) -> list[SimulationResult]:
        return [
            self.run_case(
                velocity = velocity, 
                angle_of_attack_deg=aoa, 
                internal_state = internal_state
            ) for aoa in angles_of_attack_deg
        ]

    def max_lift_case(
        self,
        *,
        velocity: float,
        angles_of_attack_deg: np.ndarray,
        internal_state: float,
        refined_step_deg: float = 0.25
    ) -> SimulationResult:
        '''
        Returns the result with the largest lift coefficient. The angles of attack are first
        searched with a coarse sweep, and then refined around the best angle, within one coarse
        step on each side.
        '''
        results = self.angle_sweep(
            velocity = velocity,
            angles_of_attack_deg = angles_of_attack_deg,
            internal_state = internal_state
        )

        best_index = max(range(len(results)), key = lambda i: results[i].cl)

        lower = angles_of_attack_deg[max(best_index - 1, 0)]
        upper = angles_of_attack_deg[min(best_index + 1, len(angles_of_attack_deg) - 1)]

        refined_angles = np.arange(lower, upper + 0.5 * refined_step_deg, refined_step_deg)

        results += self.angle_sweep(
            velocity = velocity,
            angles_of_attack_deg = refined_angles,
            internal_state = internal_state
        )

        return max(results, key = lambda result: result.cl)
