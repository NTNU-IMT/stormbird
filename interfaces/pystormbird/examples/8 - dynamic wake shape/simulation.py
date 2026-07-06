'''
Simulation setup to testing the impact of dynamic wake shapes
'''

from dataclasses import dataclass
import numpy as np

from pystormbird import SimulationResult
from pystormbird.lifting_line import Simulation
from stormbird_setup import SpatialVector
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.section_models import SectionModel, Foil
from stormbird_setup.lifting_line import ( 
    SymmetryCondition,
    DynamicWakeBuilder,
    DynamicSettings,
    QuasiSteadyWakeSettings,
    QuasiSteadySettings, 
    SimulationBuilder,
    Iterative,
    ViscousCoreLengthEvolution
)

@dataclass
class SimulationCase:
    angle_of_attack_deg: float
    wind_angle_deg: float = 45.0
    wind_speed: float = 12.0
    chord_length: float = 6.0
    span: float = 24.0
    start_height: float = 8.1
    nr_sections: int = 40
    density = 1.225
    dynamic: bool = False
    dynamic_shape: bool = False
    export_wake: bool = False

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.wind_speed**2
    
    @property
    def wind_angle(self) -> float:
        return np.radians(self.wind_angle_deg)

    def get_line_force_model(self) -> LineForceModelBuilder:
        chord_vector = SpatialVector(x=self.chord_length)
        
        line_force_model = LineForceModelBuilder(nr_sections=self.nr_sections)

        x_coordinates = [0.0, 0.0]
        y_coordinates = [-6.0, 6.0]

        for x, y in zip(x_coordinates, y_coordinates):
            wing_builder = WingBuilder(
                section_points = [
                    SpatialVector(x=x, y=y, z=self.start_height), 
                    SpatialVector(x=x, y=y, z=self.start_height + self.span)
                ],
                chord_vectors = [chord_vector, chord_vector],
                section_model = SectionModel(
                    model = Foil(
                        cd_min = 0.01,
                        mean_positive_stall_angle = np.radians(45.0), # Set large value to 'turn off' stall
                        mean_negative_stall_angle = np.radians(45.0)
                    )
                )
            )
            
            line_force_model.add_wing_builder(wing_builder)

        return line_force_model
    
    def free_stream_velocity(self):
        return SpatialVector(x=self.wind_speed)
    
    def wing_angle(self):
        return np.radians(self.wind_angle_deg - self.angle_of_attack_deg)
    
    def run(self) -> list[SimulationResult]:
        freestream_velocity = self.free_stream_velocity()

        line_force_model = self.get_line_force_model()

        

        if self.dynamic or self.dynamic_shape:
            wake = DynamicWakeBuilder(
                symmetry_condition=SymmetryCondition.Z
            )

            if not self.dynamic:
                wake.steady_state_strength_update = True

            if self.dynamic_shape:
                wake.ratio_of_wake_affected_by_induced_velocities = 1.0
                wake.shape_damping_factor = 0.0
                wake.viscous_core_length_evolution = ViscousCoreLengthEvolution.new_sin_increase(
                    last_panel_value_absolute=1.0 * self.chord_length,
                    evolution_length_factor=1.0
                )

            if self.export_wake:
                wake.wake_files_folder_path = "wake_output"
                wake.write_wake_data_to_file = True

            solver = Iterative(
                max_iterations_per_time_step=10,
                damping_factor=0.1
            )
            
            simulation_settings = DynamicSettings(
                wake = wake,
                solver = solver
            )
        else:
            solver = Iterative(
                max_iterations_per_time_step=1000,
                damping_factor=0.05
            )
            
            simulation_settings = QuasiSteadySettings(
                wake = QuasiSteadyWakeSettings(
                    symmetry_condition = SymmetryCondition.Z
                ),
                solver = solver
            )
            

        setup = SimulationBuilder(
            line_force_model = line_force_model,
            simulation_settings = simulation_settings
        )

        if self.dynamic:
            dt = 0.25 * self.chord_length / self.wind_speed
            nr_time_steps = 100
        elif self.dynamic_shape:
            dt = 0.25 * self.chord_length / self.wind_speed
            nr_time_steps = 20
        else:
            dt = 1
            nr_time_steps = 1

        simulation = Simulation(setup.to_json_string())

        freestream_velocity_points = simulation.get_freestream_velocity_points()

        freestream_velocity_list = []
        for _ in freestream_velocity_points:
            freestream_velocity_list.append(
                freestream_velocity.as_list()
            )

        nr_wings = 2
        wing_angles = np.ones(nr_wings) * self.wing_angle()

        simulation.set_local_wing_angles(wing_angles.tolist())
        simulation.set_rotation_only([0.0, 0.0, -self.wind_angle])

        result = []
        for i in range(nr_time_steps):
            result.append(
                simulation.do_step(
                    time = i * dt,
                    time_step = dt,
                    freestream_velocity = freestream_velocity_list
                )
            )

        return result
