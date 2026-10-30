'''
Plots streamlines in the flow fields exported as VTK files by run_single_rotor.py and
run_two_rotors.py, with --export-vtk. The streamlines start from line sources along the axis of each
rotor, from the foundation to the top of the rotor. The case is chosen in the same way as for the
time series in plotting.py, either with the spin ratio for the single rotor case, or with the
angular position of the interfering rotor for the two rotor case:

    python plot_streamlines.py --spin-ratio 3.0
    python plot_streamlines.py --angular-position 30 --variant lifting_line

The plot is shown directly by default. Use --save to save it to file instead, next to the VTK file.
Requires pyvista.
'''

import argparse
from pathlib import Path

import pyvista as pv
from stormbird_setup.spatial_vector import SpatialVector

import rotor_setup
from rotor_setup import Variant
from run_single_rotor import vtk_path
from run_two_rotors import two_rotors_vtk_path

NR_STREAMLINES_PER_ROTOR = 100

def plot_streamlines(
    file_path: Path,
    rotor_locations: list[SpatialVector],
    title: str,
    save: bool = False
) -> None:
    '''
    Plots streamlines from a line source along the axis of each rotor, colored by the magnitude of
    the velocity. The rotors are shown as cylinders.
    '''
    if not file_path.exists():
        raise FileNotFoundError(
            f"No VTK file found at {file_path}. Run the case with --export-vtk first."
        )

    # The fields are stored at the cell centers, but the streamlines are integrated from point data
    fields = pv.read(file_path).cell_data_to_point_data()
    fields.set_active_vectors("velocity")

    bottom = rotor_setup.FOUNDATION_HEIGHT
    top = rotor_setup.FOUNDATION_HEIGHT + rotor_setup.HEIGHT

    plotter = pv.Plotter(off_screen=save)

    for location in rotor_locations:
        source = pv.Line(
            (location.x, location.y, bottom),
            (location.x, location.y, top),
            resolution=NR_STREAMLINES_PER_ROTOR - 1
        )

        streamlines = fields.streamlines_from_source(
            source,
            vectors="velocity",
            integration_direction="both",
            max_length=2.0 * rotor_setup.TUNNEL_LENGTH
        )

        plotter.add_mesh(
            streamlines,
            scalars="velocity",
            cmap="viridis",
            line_width=2,
            scalar_bar_args={"title": "Velocity [m/s]"}
        )

        plotter.add_mesh(
            pv.Cylinder(
                center=(location.x, location.y, 0.5 * (bottom + top)),
                direction=(0.0, 0.0, 1.0),
                radius=0.5 * rotor_setup.DIAMETER,
                height=rotor_setup.HEIGHT
            ),
            color="lightgray"
        )

    plotter.add_mesh(fields.outline(), color="black")
    plotter.add_title(title, font_size=10)
    plotter.add_axes()
    plotter.camera_position = "iso"

    if save:
        plotter.screenshot(file_path.with_suffix(".png"))
    else:
        plotter.show()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--spin-ratio", type=float, default=None,
        help="Spin ratio of the single rotor case to plot. Not needed for the two rotor case."
    )
    parser.add_argument(
        "--angular-position", type=float, default=None,
        help="Plot the two rotor case with the interfering rotor at this angular position"
    )
    parser.add_argument(
        "--variant", default=rotor_setup.DEFAULT_VARIANT.value,
        choices=[variant.value for variant in Variant], help="Variant of the case to plot"
    )
    parser.add_argument(
        "--save", action="store_true", help="Save the plot to a file instead of showing it"
    )

    args = parser.parse_args()

    variant = Variant(args.variant)

    if args.angular_position is not None:
        file_path = two_rotors_vtk_path(variant, args.angular_position)
        rotor_locations = rotor_setup.two_rotors_locations(args.angular_position)
        title = f"{variant.label}, two rotors, angular position {args.angular_position}"
    elif args.spin_ratio is not None:
        file_path = vtk_path(variant, args.spin_ratio)
        rotor_locations = [SpatialVector()]
        title = f"{variant.label}, spin ratio {args.spin_ratio}"
    else:
        parser.error("Either --spin-ratio or --angular-position must be given")

    plot_streamlines(file_path, rotor_locations, title, args.save)
