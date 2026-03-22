"""
This example is intended to show how to set up "generic sail models", using the Stormbird library.
That is, the library comes with standard settings for modeling different sail types, such as 
different types of wing sails, rotor sails, and suction sails. Note, the point of the generic models
are not necessarily to be very accurate compared to any specific supplier, as all sail types have 
large variations in performance parameters. However, the settings are intended to be rough values, 
useful if one wants to make a quick comparison, for whatever reason. 

This example essentially demonstrates how to load the generic settings, and how to combine force
models with control systems in a complete sail model.
"""


from stormbird_setup.direct_setup.lifting_line import (
    SimulationBuilder, 
    QuasiSteadySettings, 
    QuasiSteadyWakeSettings, 
    SymmetryCondition, 
    CompleteSailModelBuilder
)

from stormbird_setup.simplified_setup.simple_sail_setup import SimpleSailSetup, SailType
from stormbird_setup.direct_setup.controller import ControllerBuilder
from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.wind import WindEnvironment

from pystormbird.lifting_line import CompleteSailModel
from pystormbird.wind import WindCondition

import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots

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

if __name__ == "__main__":
    sail_types_to_compare = [
        SailType.WingSailSingleElement,
        SailType.WingSailTwoElement,
        SailType.RotorSail,
        SailType.SuctionSail
    ]

    default_colors = [
        '#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd',
        '#8c564b', '#e377c2', '#7f7f7f', '#bcbd22', '#17becf'
    ]

    chord_length = 5.0
    height = 35.0
    area = chord_length * height
    deck_height = 0.0

    ship_velocity = 12.0 * 0.5144444
    wind_velocity = 8.0
    density = 1.225
    wind_directions_deg = np.arange(-180.0, 181, 4)

    fig = make_subplots(rows=2, cols=2)

    for sail_index, sail_type in enumerate(sail_types_to_compare):
        simulation_builder = SimulationBuilder(
            simulation_settings = QuasiSteadySettings(
                wake = QuasiSteadyWakeSettings(
                    symmetry_condition=SymmetryCondition.Z
                )
            )
        )

        sail = SimpleSailSetup(
            position = SpatialVector(x=0.0, y=0.0, z=deck_height),
            chord_length = chord_length,
            height = height,
            sail_type = sail_type
        )

        simulation_builder.line_force_model.add_wing_builder(sail.wing_builder())
        
        sail_type.add_default_corrections(simulation_builder)

        controller_set_points = sail.controller_set_points()
        
        max_internal_state = np.max(controller_set_points.section_model_internal_state_data)
        max_angle_of_attack = np.max(controller_set_points.angle_of_attack_data)

        model_builder = CompleteSailModelBuilder(
            lifting_line_simulation = simulation_builder,
            controller = ControllerBuilder(set_points = [controller_set_points]),
            wind_environment=WindEnvironment(),
        )

        model = CompleteSailModel(model_builder.to_json_string())
        
        wind_environment = model.get_wind_environment()
        
        # --------------- Lift and drag  --------------------------------------------------
        n_test = 20
        
        match sail_type:
            case SailType.WingSailSingleElement:
                alpha_test = np.radians(np.linspace(0, 30, n_test))
            case SailType.WingSailTwoElement:
                model.set_section_models_internal_state([max_internal_state])
                alpha_test = np.radians(np.linspace(0, 30, n_test))
            case SailType.RotorSail:
                alpha_test = np.linspace(0, 5.0, n_test)
            case SailType.SuctionSail:
                model.set_section_models_internal_state([max_internal_state])
                alpha_test = np.radians(np.linspace(0, 35, n_test))
            case _:
                raise ValueError("Undefined sail type", sail_type)
        
        cl = np.zeros(n_test)
        cd = np.zeros(n_test)
        
        for i_alpha in range(n_test):
            if sail_type == SailType.RotorSail:
                rps = revolutions_per_second_from_spin_ratio(
                    spin_ratio = alpha_test[i_alpha],
                    diameter = chord_length,
                    velocity = wind_velocity
                )
                
                model.set_section_models_internal_state([rps])
            else:
                model.set_local_wing_angles([-alpha_test[i_alpha]])
            
            wind_condition = WindCondition.new_constant(
                direction_coming_from = 0.0,
                velocity = wind_velocity
            )
            
            result = model.do_step(
                time = 0, 
                time_step = 1,
                wind_condition = wind_condition,
                ship_velocity = 0.0
            )
            
            forces = result.integrated_forces_sum()
            
            cd[i_alpha] = forces[0] / (0.5 * density * area * wind_velocity**2)
            cl[i_alpha] = forces[1] / (0.5 * density * area * wind_velocity**2)
            
        if sail_type == SailType.RotorSail:
            fig.add_trace(
                go.Scatter(
                    x = alpha_test,
                    y = cl,
                    line=dict(color=default_colors[sail_index]),
                    showlegend=False,
                    xaxis = 'x5',
                    yaxis = 'y'
                )
            )
            
            fig.add_trace(
                go.Scatter(
                    x = [max_internal_state],
                    y = [np.interp(max_internal_state, alpha_test, cl)],
                    line = dict(color=default_colors[sail_index], dash="dot"),
                    showlegend=False,
                    xaxis = 'x5',
                    yaxis = 'y'
                )
            )
        else:
            fig.add_trace(
                go.Scatter(
                    x = np.degrees(alpha_test),
                    y = cl,
                    line=dict(color=default_colors[sail_index]),
                    showlegend=False,
                ),
                row=1, col=1
            )
            
            fig.add_trace(
                go.Scatter(
                    x = np.degrees([max_angle_of_attack]),
                    y = [np.interp(max_angle_of_attack, alpha_test, cl)],
                    line=dict(color=default_colors[sail_index], dash="dot"),
                    showlegend=False,
                ),
                row=1, col=1
            )
        
        fig.add_trace(
            go.Scatter(
                x = cl,
                y = cd,
                line=dict(color=default_colors[sail_index]),
                showlegend=False
            ),
            row=1, col=2
        )
            
        
        # --------------- Compute thrust and net power ------------------------------------
        thrust = np.zeros_like(wind_directions_deg)
        thrust_coefficient = np.zeros_like(wind_directions_deg)
        propulsive_power = np.zeros_like(wind_directions_deg)
        power_net = np.zeros_like(wind_directions_deg)
        apparent_wind_direction = np.zeros_like(wind_directions_deg)

        section_model_internal_state = np.zeros_like(wind_directions_deg)

        for index, wind_dir_deg in enumerate(wind_directions_deg):
            wind_dir_rad = np.radians(wind_dir_deg)

            u_wind_apparent = ship_velocity + wind_velocity * np.cos(wind_dir_rad)
            v_wind_apparent = -wind_velocity * np.sin(wind_dir_rad)

            u_inf = np.sqrt(u_wind_apparent**2 + v_wind_apparent**2)
            
            wind_condition = WindCondition.new_constant(
                direction_coming_from = wind_dir_rad,
                velocity = wind_velocity
            )

            apparent_wind_direction[index] = wind_environment.apparent_wind_direction_from_condition_and_linear_velocity(
                condition = wind_condition,
                linear_velocity = [ship_velocity, 0.0, 0.0]
            )
            
            optimizing = True
            loading = 1.0
            delta_loading = 0.05
            min_loading = 0.05
            
            previous_power_net = -np.inf
            
            
            while optimizing and loading > min_loading:
                model.apply_controller(
                    time = 0, 
                    time_step = 1,
                    wind_condition = wind_condition,
                    ship_velocity = ship_velocity,
                    controller_loading = loading
                )
                
                result = model.do_step(
                    time = 0, 
                    time_step = 1,
                    wind_condition = wind_condition,
                    ship_velocity = ship_velocity
                )
                
                local_thrust = -result.integrated_forces_sum()[0]
                local_propulsive_power = local_thrust * ship_velocity
                local_power_net = local_propulsive_power - result.input_power_sum()
                
                if local_power_net > previous_power_net:
                    thrust[index] = local_thrust
                    
                    thrust_coefficient[index] = thrust[index] / (0.5 * density * area * u_inf**2)
        
                    propulsive_power[index] = local_propulsive_power
                    power_net[index] = local_power_net
                    
                    loading -= delta_loading
                else:
                    optimizing = False
                    
                previous_power_net = local_power_net
            
        if sail.sail_type.consumes_power():
            name = f"{sail_type.value} effective"
        else:
            name = f"{sail_type.value}"
            
        # Add thrust coefficient trace
        fig.add_trace(
            go.Scatter(
                x=np.degrees(apparent_wind_direction),
                y=thrust_coefficient,
                mode='lines',
                name=name,
                line=dict(color=default_colors[sail_index]),
                showlegend=True,
                legendgroup=f"group{sail_index}"
            ),
            row=2, col=1
        )

        # Add power net trace
        fig.add_trace(
            go.Scatter(
                x=np.degrees(apparent_wind_direction),
                y=power_net / area,
                mode='lines',
                name=name,
                line=dict(color=default_colors[sail_index]),
                showlegend=False,
                legendgroup=f"group{sail_index}"
            ),
            row=2, col=2
        )

        if sail.sail_type.consumes_power():
            # Add propulsive power trace
            fig.add_trace(
                go.Scatter(
                    x=np.degrees(apparent_wind_direction),
                    y=propulsive_power / area,
                    mode='lines',
                    name=f"{sail_type.value} propulsive",
                    line=dict(color=default_colors[sail_index], dash='dash'),
                    showlegend=True,
                    legendgroup=f"group{sail_index}"
                ),
                row=2, col=2
            )
            
    fig.add_trace(
        go.Scatter(
            x=[None],
            y=[None],
            line=dict(color="grey", dash="dot"),
            name="Max controller set point",
            showlegend=True
        )
    )

    # Update layout
    fig.update_xaxes(title_text="Angle of attack [deg]", row=1, col=1)
    fig.update_xaxes(title_text="Lift coefficient", row=1, col=2)
    
    fig.update_xaxes(title_text="Apparent wind direction (deg)", row=2, col=1, range=[0, 180])
    fig.update_xaxes(title_text="Apparent wind direction (deg)", row=2, col=2, range=[0, 180])
    
    fig.update_yaxes(title_text="Lift coefficient", row=1, col=1)
    fig.update_yaxes(title_text="Drag coefficient", row=1, col=2)
    
    fig.update_yaxes(title_text="Thrust coefficient", row=2, col=1)
    fig.update_yaxes(title_text="Power per area (W/m^2)", row=2, col=2)

    ship_velocity_knots = ship_velocity / 0.5144444
    fig.add_annotation(
        text=f"V<sub>wind</sub> = {wind_velocity:.1f} m/s, V<sub>ship</sub> = {ship_velocity_knots:.0f} kn",
        xref="x4 domain",
        yref="y4 domain",
        x=0.5,
        y=1.02,
        showarrow=False,
        font=dict(size=12),
        xanchor="center",
        yanchor="bottom"
    )
    
    fig.update_layout(
        xaxis5=dict(
            title="Spin ratio",
            overlaying='x',
            side='top',
            anchor='y'
        )
    )

    fig.update_layout(
        height=960,
        width=960,
        margin=dict(t=20),
        legend=dict(
            orientation="h",
            yanchor="bottom",
            y=-0.2,
            xanchor="center",
            x=0.5
        )
    )
    
    fig.write_image("compare_sail_types.pdf")
    fig.write_image("compare_sail_types.svg")
    fig.write_image("compare_sail_types.png")

    fig.show()
