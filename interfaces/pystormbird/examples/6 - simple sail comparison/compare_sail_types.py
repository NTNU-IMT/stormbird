
from stormbird_setup.direct_setup.lifting_line.simulation_builder import SimulationBuilder
from stormbird_setup.direct_setup.lifting_line.complete_sail_model import CompleteSailModelBuilder
from stormbird_setup.direct_setup.controller import ControllerBuilder

from stormbird_setup.simplified_setup.simple_sail_setup import SimpleSailSetup, SailType
from stormbird_setup.direct_setup.spatial_vector import SpatialVector
from stormbird_setup.direct_setup.wind import WindEnvironment

from pystormbird.lifting_line import CompleteSailModel
from pystormbird.wind import WindCondition

import numpy as np
import plotly.graph_objects as go
from plotly.subplots import make_subplots

if __name__ == "__main__":
    sail_types_to_compare = [
        SailType.WingSailSingleElement,
        SailType.WingSailTwoElement,
        SailType.RotorSail,
        SailType.SuctionSail
    ]

    # Plotly default color sequence
    default_colors = [
        '#1f77b4', '#ff7f0e', '#2ca02c', '#d62728', '#9467bd',
        '#8c564b', '#e377c2', '#7f7f7f', '#bcbd22', '#17becf'
    ]

    chord_length = 5.0
    height = 35.0
    area = chord_length * height
    deck_height = 10.0

    ship_velocity = 12.0 * 0.5144444
    wind_velocity = 8.0
    density = 1.225
    wind_directions_deg = np.arange(-180.0, 181, 2)

    fig = make_subplots(
        rows=1, cols=2,
    )

    for sail_index, sail_type in enumerate(sail_types_to_compare):
        simulation_builder = SimulationBuilder()

        sail = SimpleSailSetup(
            position = SpatialVector(x=0.0, y=0.0, z=deck_height),
            chord_length = chord_length,
            height = height,
            sail_type = sail_type
        )

        simulation_builder.line_force_model.add_wing_builder(sail.wing_builder())

        controller_set_points = sail.controller_set_points()

        model_builder = CompleteSailModelBuilder(
            lifting_line_simulation=simulation_builder,
            controller=ControllerBuilder(set_points = [controller_set_points]),
            wind_environment=WindEnvironment(),
        )

        model = CompleteSailModel(model_builder.to_json_string())

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

            apparent_wind_direction[index] = np.arctan2(-v_wind_apparent, u_wind_apparent)
            
            wind_condition = WindCondition.new_constant(
                direction_coming_from = wind_dir_rad,
                velocity = wind_velocity
            )
            optimizing = True
            loading = 1.0
            delta_loading = 0.05
            
            previous_power_net = -np.inf
            
            while optimizing and loading > 0.0:
                result = model.do_step(
                    time = 0, 
                    time_step = 1,
                    wind_condition=wind_condition,
                    ship_velocity = ship_velocity,
                    controller_loading = loading
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

            section_model_internal_state[index] = model.section_models_internal_state()[0]
            
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
            row=1, col=1
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
            row=1, col=2
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
                row=1, col=2
            )

    # Update layout
    fig.update_xaxes(title_text="Apparent wind direction (deg)", row=1, col=1, range=[0, 180])
    fig.update_xaxes(title_text="Apparent wind direction (deg)", row=1, col=2, range=[0, 180])
    
    fig.update_yaxes(title_text="Thrust / (0.5 density * area * velocity^2)", row=1, col=1)
    fig.update_yaxes(title_text="Power per area (W/m^2)", row=1, col=2)

    fig.update_layout(
        title=dict(
            text=f"Ship velocity: {ship_velocity/0.5144444:.1f} kn, Wind velocity: {wind_velocity:.1f} m/s",
            x=0.5,
            xanchor='center'
        ),
        height=600,
        width=960,
        hovermode='x unified',
        legend=dict(
            orientation="h",
            yanchor="bottom",
            y=-0.3,
            xanchor="center",
            x=0.5
        )
    )
    
    fig.write_image("compare_sail_types.pdf")
    fig.write_image("compare_sail_types.svg")
    fig.write_image("compare_sail_types.png")

    fig.show()
