'''
Simulation setup to reproduce the lifting line results from the paper "Actuator line for wind 
propulsion modelling", found here: 
https://www.researchgate.net/publication/374976524_Actuator_Line_for_Wind_Propulsion_Modelling
'''

from dataclasses import dataclass
import numpy as np

from pystormbird.lifting_line import Simulation
from stormbird_setup.direct_setup import SpatialVector
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.section_models import SectionModel, Foil
from stormbird_setup.direct_setup.lifting_line import (
    SimpleIterative, 
    SymmetryCondition,
    QuasiSteadyWakeSettings,
    QuasiSteadySettings, 
    SimulationBuilder
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

    @property
    def force_factor(self) -> float:
        return 0.5 * self.chord_length * self.span * self.density * self.wind_speed**2
    
    @property
    def wind_angle(self) -> float:
        return np.radians(self.wind_angle_deg)

    def get_line_force_model(self) -> dict:
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
    
    def run(self):
        freestream_velocity = self.free_stream_velocity()

        line_force_model = self.get_line_force_model()

        setup = SimulationBuilder(
            line_force_model = line_force_model,
            simulation_settings = QuasiSteadySettings(
                wake = QuasiSteadyWakeSettings(
                    symmetry_condition = SymmetryCondition.Z
                )
            )
        )

        dt = 0.1
        nr_time_steps = 100

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

        for i in range(nr_time_steps):
            result = simulation.do_step(
                time = i * dt,
                time_step = dt,
                freestream_velocity = freestream_velocity_list
            )

        return result


