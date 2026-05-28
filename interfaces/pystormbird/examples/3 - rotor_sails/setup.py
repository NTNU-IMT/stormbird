


import numpy as np

from stormbird_setup.spatial_vector import SpatialVector
from stormbird_setup.section_models import SectionModel
from stormbird_setup.line_force_model import LineForceModelBuilder, WingBuilder
from stormbird_setup.lifting_line.simulation_builder import SimulationBuilder, QuasiSteadySettings, DynamicSettings
from stormbird_setup.lifting_line.solver import Linearized, SimpleIterative
from stormbird_setup.lifting_line.wake import QuasiSteadyWakeSettings, SymmetryCondition, DynamicWakeBuilder, ViscousCoreLengthEvolution
from stormbird_setup.utils import revolutions_per_second_from_spin_ratio

from stormbird_setup.lifting_line.velocity_corrections import VelocityCorrections, VelocityCorrectionType

from stormbird_setup.simplified_setup.single_wing_simulation import SolverType

from stormbird_setup.circulation_corrections import CirculationCorrectionBuilder

from pystormbird.lifting_line import Simulation

from tqdm import tqdm

def simulate_single_case(
    *,
    diameter: float,
    height: float,
    foundation_height: float,
    rotor_x_locations: list[float],
    rotor_y_locations: list[float],
    spin_ratio: float,
    solver_type: SolverType = SolverType.Linearized,
    max_induced_velocity_ratio: float = 2.0,
    smoothing_length: float = 0.0,
    wind_direction_deg: float = 0.0,
    virtual_extension_factor_top: float = 0.0,
    dynamic: bool = False,
    dynamic_shape: bool = False,
    write_wake: bool = False
) -> list[dict]:
    velocity = 8.0
    density  = 1.225
    nr_sections = 20
    
    non_zero_circulation_at_ends = (True, True)
    
    nr_sails = len(rotor_x_locations)
    
    force_factor = 0.5 * diameter * height * density * velocity**2
    
    chord_vector = SpatialVector(x=diameter)
    
    wing_builders = []
    
    for x, y in zip(rotor_x_locations, rotor_y_locations):
        section_points = [
            SpatialVector(x=x, y=y, z=foundation_height),
            SpatialVector(x=x, y=y, z=foundation_height + height)
        ]

        chord_vectors = [
            chord_vector,
            chord_vector
        ]

        line_segment_is_virtual = None
        if virtual_extension_factor_top > 0.0:
            section_points.append(
                SpatialVector(x=x, y=y, z=foundation_height + height + virtual_extension_factor_top * diameter)
            )

            chord_vectors.append(chord_vector)

            line_segment_is_virtual = [False, True]
            
        
        wing_builders.append(
            WingBuilder(
                section_points = section_points,
                chord_vectors = chord_vectors,
                line_segment_is_virtual = line_segment_is_virtual,
                section_model = SectionModel.rotor_sail_deybach_2024(),
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

    if dynamic:
        solver = SimpleIterative(
            max_iterations_per_time_step = 20,
            damping_factor = 0.2,
            start_with_linearized_solution = False
        )

        # Dynamic wake with full dynamic shape, but also with some stabilizing numerical techniques
        # This includes
        # - Strong "shape_damping", to limit how quickly the wake shape reacts to new induced velocities
        # - A viscous core length that increases rapidly as the wake panels move away from the wings

        if dynamic_shape:
            ratio_of_wake_affected_by_induced_velocities = 1.0
        else:
            ratio_of_wake_affected_by_induced_velocities = 0.0
        
        wake = DynamicWakeBuilder(
            ratio_of_wake_affected_by_induced_velocities=ratio_of_wake_affected_by_induced_velocities,
            shape_damping_factor=0.5,
            nr_panels_per_line_element=200,
            viscous_core_length_evolution=ViscousCoreLengthEvolution.new_sin_increase(
                last_panel_value_absolute=1.5 * diameter,
                evolution_length_factor=0.3
            ),
            symmetry_condition = SymmetryCondition.Z
        )

        if write_wake:
            wake.wake_files_folder_path = "wake_files"
            wake.write_wake_data_to_file = True

        simulation_builder = SimulationBuilder(
            line_force_model = line_force_model,
            simulation_settings = DynamicSettings(
                solver = solver,
                wake = wake
            )
        )
    else:
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

    if dynamic:
        dt = 0.25 * diameter / velocity
        nr_time_steps = int(
            1.5 * simulation_builder.simulation_settings.wake.nr_panels_per_line_element
        )
        
        t = 0.0

        for i_t in tqdm(range(nr_time_steps)):
            result = simulation.do_step(
                time = t, 
                time_step = dt, 
                freestream_velocity = freestream_velocity_list
            )

            t += dt
    else:
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
