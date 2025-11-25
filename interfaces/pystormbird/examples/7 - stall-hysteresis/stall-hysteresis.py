

import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

from tqdm import tqdm

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.section_models import SectionModel, Foil
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, DynamicSettings, QuasiSteadySettings
from stormbird_setup.direct_setup.lifting_line.solver import SimpleIterative
from stormbird_setup.direct_setup.lifting_line.wake import DynamicWakeBuilder, QuasiSteadyWakeSettings, SymmetryCondition

from stormbird_setup.direct_setup.circulation_corrections import CirculationCorrectionBuilder

from pystormbird.lifting_line import Simulation



if __name__ == "__main__":
    chord = 0.2 + 0.15
    start_height = 0.1
    span = 1.0
    nr_sections = 32
    velocity = 8.0
    density = 1.225
    
    force_factor = 0.5 * density * chord * span * velocity**2
    
    wing_builder = WingBuilder(
        section_points=[
            SpatialVector(z=start_height),
            SpatialVector(z=start_height + span)
        ],
        chord_vectors = [
            SpatialVector(x=chord),
            SpatialVector(x=chord)
        ],
        section_model = SectionModel(
            model=Foil(
                cl_zero_angle=1.3,
                mean_positive_stall_angle=np.radians(14.0),
                stall_range = np.radians(10.0),
                cl_max_after_stall = 1.0
            )
        )
    )
    
    line_force_model = LineForceModelBuilder(
        nr_sections = nr_sections, 
        density = density,
        circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(
            smoothing_length_factor = 0.07
        )
    )
    
    line_force_model.add_wing_builder(wing_builder)
    
    wake = QuasiSteadyWakeSettings(
        symmetry_condition=SymmetryCondition.Z,
    )
    
    solver = SimpleIterative(
        max_iterations_per_time_step=500,
        damping_factor = 0.02
    )
    
    simulation_builder = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = QuasiSteadySettings(
            wake = wake,
            solver = solver
        ),
    )
    
    simulation = Simulation(simulation_builder.to_json_string())
    
    velocity_points = simulation.get_freestream_velocity_points()
    
    freestream_velocity = []
    for _point in velocity_points:
        freestream_velocity.append([velocity, 0.0, 0.0])
        
    dalpha_dt = np.radians(0.5)
    
    end_time = np.radians(100.0) / dalpha_dt
    time_step = 0.25 * chord / velocity
    
    nr_time_steps = int(end_time / time_step)
    
    time = 0.0
    local_wing_angle = np.radians(5.0)
    
    cl = []
    angles_of_attack = []
    
    turned_around = False
    
    direction_indices = []
    
    for time_index in tqdm(range(nr_time_steps)):
        if np.abs(local_wing_angle) > np.radians(35) and not turned_around:
           turned_around = True 
        
        if turned_around:
            local_wing_angle += dalpha_dt * time_step
            direction_indices.append(-1)
        else:
            local_wing_angle -= dalpha_dt * time_step
            direction_indices.append(1)
            
        angles_of_attack.append(-local_wing_angle)
        
        simulation.set_local_wing_angles([local_wing_angle])
        
        #simulation.reset_previous_circulation_strength()
        
        result = simulation.do_step(
            time = time,
            time_step = time_step,
            freestream_velocity = freestream_velocity,
        )
        
        forces = result.integrated_forces_sum()
        
        cl.append(forces[1] / force_factor)
        
        time += time_step
        
    increasing_indices = np.where(np.array(direction_indices) == 1)
    decreasing_indices = np.where(np.array(direction_indices) == -1)
    
    plt.plot(np.degrees(angles_of_attack)[increasing_indices], np.array(cl)[increasing_indices], label="LL, increasing")
    plt.plot(np.degrees(angles_of_attack)[decreasing_indices], np.array(cl)[decreasing_indices], label="LL, decreasing")
    
    increasing_cl = pd.read_csv("increasing_cl.csv")
    decreasing_cl = pd.read_csv("decreasing_cl.csv")
    
    plt.plot(increasing_cl["angle"], increasing_cl["cl"], '--', label="Exp, increasing")
    plt.plot(decreasing_cl["angle"], decreasing_cl["cl"], '--', label="Exp, decreasing")
    
    plt.xlim(-10, 35)
    plt.ylim(0, 2.0)
    
    plt.legend()
    
    plt.show()
