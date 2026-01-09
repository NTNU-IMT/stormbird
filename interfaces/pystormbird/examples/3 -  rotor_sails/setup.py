

from typing_extensions import Dict
import numpy as np

from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.section_models import SectionModel, RotatingCylinder
from stormbird_setup.direct_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings
from stormbird_setup.direct_setup.lifting_line.solver import Linearized, SimpleIterative
from stormbird_setup.direct_setup.lifting_line.wake import QuasiSteadyWakeSettings, SymmetryCondition, ViscousCoreLength, ViscousCoreLengthType

from stormbird_setup.direct_setup.lifting_line.velocity_corrections import VelocityCorrections, VelocityCorrectionType

from stormbird_setup.simplified_setup.single_wing_simulation import SolverType

from stormbird_setup.direct_setup.circulation_corrections import CirculationCorrectionBuilder

from pystormbird.lifting_line import Simulation

# These settings are used to generate multiple versions of the simulation, for test purposes
TEST_SETTINGS = {
    "max_induced_velocity_ratios": [0.0, 2.0, 2.0, 0.0],
    "smoothing_lengths": [0.0, 0.0, 0.0, 0.05],
    "solver_types": [SolverType.SimpleIterative, SolverType.SimpleIterative, SolverType.Linearized, SolverType.SimpleIterative]
}

def get_section_model():
    '''
    Returns the section model. Slightly modified from the default. The lift data is tuned to match 
    the 3D CFD results that is used for comparison in this case, which is often necessary.
    '''

    return SectionModel(
        model = RotatingCylinder(
            spin_ratio_data = [0.0, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0],
            cl_data = [0.0, 1.22, 2.56, 5.93, 8.87, 9.56, 10.22, 12.0, 13.00]
        )
    )

def revolutions_per_second_from_spin_ratio(
    *,
    spin_ratio: float,
    diameter: float,
    velocity: float
):
    '''
    Helper function to convert spin ratio to revolutions per second
    '''
    circumference = np.pi * diameter
    tangential_velocity = velocity * spin_ratio
            
    revolutions_per_second = -tangential_velocity / circumference 

    return revolutions_per_second

def simulate_single_case(
    *,
    rotor_x_locations: list[float],
    rotor_y_locations: list[float],
    spin_ratio: float,
    solver_type: SolverType = SolverType.Linearized,
    max_induced_velocity_ratio: float = 2.0,
    smoothing_length: float = 0.0,
    wind_direction_deg = 0.0
) -> list[dict]:

    diameter = 5.0
    height   = 35.0
    velocity = 8.0
    density  = 1.225
    nr_sections = 32
    
    non_zero_circulation_at_ends = (True, False)
    
    nr_sails = len(rotor_x_locations)
    
    force_factor = 0.5 * diameter * height * density * velocity**2
    
    chord_vector = SpatialVector(x=diameter)
    
    wing_builders = []
    
    for x, y in zip(rotor_x_locations, rotor_y_locations):        
        wing_builders.append(
            WingBuilder(
                section_points = [
                    SpatialVector(x=x, y=y, z=0.0),
                    SpatialVector(x=x, y=y, z=height)
                ],
                chord_vectors = [
                    chord_vector,
                    chord_vector
                ],
                section_model = get_section_model(),
                non_zero_circulation_at_ends = non_zero_circulation_at_ends
            )
        )
    
    if smoothing_length > 0.0:
        circulation_correction = CirculationCorrectionBuilder.new_gaussian_smoothing(smoothing_length)
    else:
        circulation_correction = CirculationCorrectionBuilder()  

    line_force_model = LineForceModelBuilder(
        nr_sections = nr_sections,
        density = density,
        circulation_correction = circulation_correction
    )
    
    for wing_builder in wing_builders:
        line_force_model.add_wing_builder(wing_builder)
    
    match solver_type:
        case SolverType.SimpleIterative:
            solver = SimpleIterative(
                max_iterations_per_time_step = 1000,
                damping_factor = 0.05,
                start_with_linearized_solution = True
            )
        case SolverType.Linearized:
            solver = Linearized()
            
    wake = QuasiSteadyWakeSettings(
        symmetry_condition=SymmetryCondition.Z
    )
    
    simulation_builder = SimulationBuilder(
        line_force_model = line_force_model,
        simulation_settings = QuasiSteadySettings(
            solver = solver,
            wake = wake
        )
    )

    if max_induced_velocity_ratio > 0.0:
        simulation_builder.simulation_settings.solver.velocity_corrections = VelocityCorrections(
            type = VelocityCorrectionType.MaxInducedVelocityMagnitudeRatio,
            value = max_induced_velocity_ratio
        )

    simulation = Simulation(
        simulation_builder.to_json_string()
    )
    
    velocity_x = velocity * np.cos(np.radians(wind_direction_deg))
    velocity_y = velocity * np.sin(np.radians(wind_direction_deg))

    freestream_velocity_points = simulation.get_freestream_velocity_points()

    freestream_velocity_list = []
    for _ in freestream_velocity_points:
        freestream_velocity_list.append(
            [velocity_x, velocity_y, 0.0]
        )


    section_model_internal_state = revolutions_per_second_from_spin_ratio(
        spin_ratio=spin_ratio,
        diameter=diameter,
        velocity=velocity
    )

    simulation.set_section_models_internal_state([section_model_internal_state] * nr_sails)

    result = simulation.do_step(
        time = 0.0, 
        time_step = 1.0, 
        freestream_velocity = freestream_velocity_list
    )
    
    out = []
    
    circulation_strength_total = np.array(result.force_input.circulation_strength)
    angles_of_attack_total = np.array(result.force_input.angles_of_attack)
    
    for wing_index in range(nr_sails):
        force = result.integrated_forces[wing_index].total
    
        cx = force[0] / force_factor
        cy = force[1] / force_factor
        
        out.append(
            {
                "cx": cx,
                "cy": cy,
                "circulation_strength": circulation_strength_total[wing_index * nr_sections: (wing_index + 1) * nr_sections],
                "angles_of_attack": angles_of_attack_total[wing_index * nr_sections: (wing_index + 1) * nr_sections],
            }
        )

    return out
