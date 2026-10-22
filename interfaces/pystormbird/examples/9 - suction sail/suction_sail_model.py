'''
Functions for setting up and running simulations of the three-dimensional suction sail model from
the paper, shared by the scripts in this example.
'''

from dataclasses import dataclass

import numpy as np

from stormbird_setup.section_models import (
    SectionModel,
    ActiveAirFlowFoil,
    VaryingFoil,
    Foil,
    InternalStateConversion,
    InternalStateConversionType,
    InternalFlow,
)
from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings
from stormbird_setup.lifting_line.solver import Iterative
from stormbird_setup.lifting_line.wake import QuasiSteadyWakeSettings, SymmetryCondition

from pystormbird.lifting_line import Simulation

from suction_sail_data import CQ_VALUES_3D

@dataclass
class SimulationResult:
    cl: float
    cd: float
    cp: float
    section_internal_states: np.ndarray
    section_angles_of_attack: np.ndarray
    section_velocities: np.ndarray
    section_lift_coefficients: np.ndarray
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
    chord_length: float = 1.0
    height: float = 4.0
    nr_sections: int = 32
    virtual_extension_factor: float = 0.1
    smoothing_length_factor: float = 0.1
    density: float = 1.225
    # When the internal flow model is not set, the internal state is the suction coefficient.
    # Otherwise, it is the input power to the fan.
    internal_flow: InternalFlow | None = None
    fan_efficiency: float = 0.7

    @property
    def area(self) -> float:
        return self.chord_length * self.height

    def get_varying_foil(self) -> VaryingFoil:
        # The first foil is for zero suction, which is needed for low power. There is no 3D data
        # without suction, so the lift is fitted to the 2D data, while the drag is assumed to be
        # high, as for the other suction coefficients.
        internal_state_data = [0.0] + CQ_VALUES_3D
        cl_zero_angle = [1.8, 2.5, 3.3, 3.55]
        cl_initial_slope = np.array([1.08, 1.25, 1.5, 1.65]) * 2*np.pi
        cd_min = [0.4, 0.3, 0.2, 0.2]
        mean_stall_angle = np.radians([17.0, 34, 37, 39.5])
        stall_range = np.radians([6.0, 10.0, 10.0, 10])
        cl_slope_reduction_factor_positive = [0.5, 0.4, 0.5, 0.5]
        cl_slope_reduction_onset_angle_positive = np.radians([8.0, 15.0, 12.0, 12.0])
        cd_second_order_factor = [0.12]*4

        nr_data = len(internal_state_data)

        foils_data = []
        for i in range(nr_data):
            foils_data.append(
                Foil(
                    cl_zero_angle = cl_zero_angle[i],
                    cl_initial_slope = cl_initial_slope[i],
                    cd_min = cd_min[i],
                    mean_positive_stall_angle = mean_stall_angle[i],
                    mean_negative_stall_angle = float(np.radians(30.0)),
                    stall_range = stall_range[i],
                    cl_slope_reduction_factor_positive = cl_slope_reduction_factor_positive[i],
                    cl_slope_reduction_onset_angle_positive = cl_slope_reduction_onset_angle_positive[i],
                    cd_second_order_factor = cd_second_order_factor[i]
                )
            )

        return VaryingFoil(
            internal_state_data = internal_state_data,
            foils_data = foils_data
        )

    def get_section_model(self) -> SectionModel:
        if self.internal_flow is None:
            internal_state_conversion = InternalStateConversion(
                conversion_type = InternalStateConversionType.SuctionCoefficientDirect
            )
        else:
            internal_state_conversion = InternalStateConversion.new_power_to_internal_flow(
                internal_flow = self.internal_flow,
                fan_efficiency = self.fan_efficiency
            )

        return SectionModel(
            model = ActiveAirFlowFoil(
                varying_foil = self.get_varying_foil(),
                internal_state_conversion = internal_state_conversion
            )
        )

    def get_line_force_model(self) -> LineForceModelBuilder:
        chord_vector = SpatialVector(x=self.chord_length)
    
        virtual_extension_length = self.virtual_extension_factor * self.height
    
        # The wing is oriented along the z-axis, starting at the symmetry plane. The circulation is not
        # zero at the bottom of the wing, as the wing continues as a mirror image below the plane.
        # A virtual extension is added at the top to increase induced drag.
        wing_builder = WingBuilder(
            section_points = [
                SpatialVector(x=0.0, y=0.0, z=0.0),
                SpatialVector(x=0.0, y=0.0, z=self.height),
                SpatialVector(x=0.0, y=0.0, z=self.height + virtual_extension_length),
            ],
            chord_vectors = [chord_vector, chord_vector, chord_vector],
            section_model = self.get_section_model(),
            non_zero_circulation_at_ends = (True, False),
            line_segment_is_virtual = [False, True],
        )
    
        line_force_model = LineForceModelBuilder(
            nr_sections = self.nr_sections,
            density = self.density,
            circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(
                self.smoothing_length_factor
            )
        )
    
        line_force_model.add_wing_builder(wing_builder)

        return line_force_model

    def get_simulation(self) -> Simulation:
        simulation_settings = QuasiSteadySettings(
            solver = Iterative(
                max_iterations_per_time_step = 1000,
                damping_factor = 0.05,
                start_with_linearized_solution = True
            ),
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

        section_velocities = np.linalg.norm(np.array(result.force_input.velocity), axis=1)
        circulation_strength = np.array(result.force_input.circulation_strength)
    
        return SimulationResult(
            cl = force[1] / force_factor,
            cd = force[0] / force_factor,
            cp = result.input_power[0] / (force_factor * velocity),
            section_internal_states = np.array(result.force_input.section_models_internal_state),
            section_angles_of_attack = np.array(result.force_input.angles_of_attack),
            section_velocities = section_velocities,
            # From the Kutta-Joukowski theorem, as the magnitude, i.e., towards the suction side
            section_lift_coefficients = np.abs(
                2.0 * circulation_strength / (section_velocities * self.chord_length)
            ),
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
