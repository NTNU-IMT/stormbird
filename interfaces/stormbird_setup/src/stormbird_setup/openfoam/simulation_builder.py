"""
Copyright (C) 2026, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

import math
import shutil
import subprocess
from importlib.resources import as_file, files
from pathlib import Path

import numpy as np
from pydantic import Field, field_validator, model_validator

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector
from ..wind import WindCondition, WindEnvironment
from ..stormflow.geometry import GeometryBuilder, Sphere, Cuboid, Disk, TriangleMeshBuilder
from ..actuator_line import ActuatorLineBuilder
from ..stormflow.turbulence import IntensityAndViscosityRatio
from ..stormflow.simulation_builder import BoundaryConditionType
from .grid import GridBuilder
from .time_control import SteadyTimeControl, DynamicTimeControl
from .results import ActuatorLineResults
from .foam_file import FoamDict, FoamValue, read_foam_file, write_foam_file

# Height above the water plane where the true wind velocity is used as reference for the turbulence
TURBULENCE_REFERENCE_HEIGHT = 10.0

# Number of times the results are written during a simulation
NR_WRITES = 10

# Number of heights in the table of apparent wind velocities at the vertical sides of the domain
NR_WIND_TABLE_HEIGHTS = 200

# Faces of the wind tunnel patches in the blockMeshDict, using the vertex order of the base folder
WIND_TUNNEL_PATCH_FACES: dict[str, list[str]] = {
    "inlet": ["(0 4 7 3)"],
    "outlet": ["(1 2 6 5)"],
    "tunnel_walls": ["(0 1 5 4)", "(2 3 7 6)", "(0 3 2 1)", "(4 5 6 7)"],
}

# The patch in the base folder that each wind tunnel patch takes its boundary conditions from
WIND_TUNNEL_PATCH_SOURCES: dict[str, str] = {
    "inlet": "vertical_sides",
    "outlet": "z_max",
    "tunnel_walls": "z_min",
}


class SimulationBuilder(StormbirdSetupBaseModel):
    '''
    User level setup of an OpenFOAM case for actuator line simulations. The interface is kept as
    similar as possible to the Stormflow simulation builder. Settings that are not exposed here, such
    as schemes, solvers, turbulence model and viscosity, are taken from the base folder.
    '''
    grid: GridBuilder
    wind_condition: WindCondition
    linear_velocity: SpatialVector
    actuator_line: ActuatorLineBuilder | None = None
    geometries: list[GeometryBuilder] = Field(default_factory=list)
    slip_geometries: list[GeometryBuilder] = Field(default_factory=list)
    wind_environment: WindEnvironment = Field(default_factory=WindEnvironment)
    inlet_turbulence: IntensityAndViscosityRatio = Field(default_factory=IntensityAndViscosityRatio)
    """
    Turbulence in the inflow, using the true wind velocity at 10 m height as the reference velocity,
    or the linear velocity if there is no true wind. The values are the same at all heights.
    """
    time_control: SteadyTimeControl | DynamicTimeControl = Field(default_factory=SteadyTimeControl)
    boundary_condition_type: BoundaryConditionType = BoundaryConditionType.WindInletOutlet
    """
    How the boundary conditions are assigned to the faces of the domain. `WindTunnel` requires the
    apparent wind to be along the positive x axis.
    """
    base_folder: str | None = None
    """Path to the OpenFOAM base case to start from. None means the base folder in this package."""

    @field_validator("geometries", "slip_geometries")
    @classmethod
    def check_geometries(cls, geometries: list[GeometryBuilder]) -> list[GeometryBuilder]:
        for geometry in geometries:
            shape = geometry.geometry

            if isinstance(shape, Disk):
                if shape.fillet_radius != 0.0:
                    raise ValueError(
                        "Disks with a fillet radius are not supported in OpenFOAM cases. Set "
                        "fillet_radius to 0.0"
                    )

                if shape.thickness <= 0.0:
                    raise ValueError(
                        "Disks in OpenFOAM cases must have a thickness larger than zero, as they are "
                        "meshed as solid cylinders"
                    )

        return geometries

    @field_validator("wind_condition")
    @classmethod
    def check_wind_condition(cls, wind_condition: WindCondition) -> WindCondition:
        gusts = (
            wind_condition.parallel_gust,
            wind_condition.perpendicular_gust,
            wind_condition.vertical_gust
        )

        if any(gust is not None for gust in gusts):
            raise ValueError("Gusts are not supported in OpenFOAM cases")

        return wind_condition

    @field_validator("wind_environment")
    @classmethod
    def check_wind_environment(cls, wind_environment: WindEnvironment) -> WindEnvironment:
        if wind_environment.up_direction.as_tuple() != (0.0, 0.0, 1.0):
            raise ValueError("The up direction must be (0, 0, 1) in OpenFOAM cases")

        if wind_environment.inflow_corrections is not None:
            raise ValueError("Inflow corrections are not supported in OpenFOAM cases")

        return wind_environment

    @model_validator(mode="after")
    def check_wind_tunnel_inflow(self) -> "SimulationBuilder":
        '''
        The wind tunnel boundary conditions only make sense if the flow enters the domain through
        the inlet at the start of the x axis, and does not flow into the walls. Uses the same check
        as Stormflow, where the components are compared to the largest inlet velocity, as the
        velocity may be zero at some heights.
        '''
        if self.boundary_condition_type != BoundaryConditionType.WindTunnel:
            return self

        velocities = [velocity for _, velocity in self._apparent_wind_profile()]

        max_velocity = max(math.sqrt(sum(c**2 for c in velocity)) for velocity in velocities)

        if max_velocity == 0.0:
            raise ValueError("The wind tunnel boundary conditions require a non-zero inlet velocity")

        tolerance = 1e-3 * max_velocity

        for velocity in velocities:
            x, y, z = velocity

            if x <= -tolerance or abs(y) >= tolerance or abs(z) >= tolerance:
                raise ValueError(
                    "The wind tunnel boundary conditions require the apparent wind to be along the "
                    "positive x axis. Check the wind direction and the linear velocity. Got an "
                    f"inlet velocity of {velocity}"
                )

        return self

    def inlet_patch_name(self) -> str:
        '''
        The name of the patch where the apparent wind flows into the domain
        '''
        if self.boundary_condition_type == BoundaryConditionType.WindTunnel:
            return "inlet"

        return "vertical_sides"

    def representative_length(self) -> float:
        '''
        The length that all resolutions in the grid are measured relative to
        '''
        if self.grid.representative_length is not None:
            return self.grid.representative_length

        if self.actuator_line is None:
            raise ValueError(
                "grid.representative_length must be specified when there is no actuator line model"
            )

        chord_lengths = [
            chord_vector.length()
            for wing_builder in self.actuator_line.line_force_model.wing_builders
            for chord_vector in wing_builder.chord_vectors
        ]

        return sum(chord_lengths) / len(chord_lengths)

    def write_foam_case(self, path: Path | str) -> None:
        '''
        Writes an OpenFOAM case to the given path, by copying the base folder and applying the
        settings in this builder. The path must not already exist.
        '''
        case_path = Path(path)

        if self.base_folder is None:
            with as_file(files("stormbird_setup.openfoam") / "base_folder") as base_folder:
                shutil.copytree(base_folder, case_path)
        else:
            shutil.copytree(self.base_folder, case_path)

        self._convert_to_lf_line_endings(case_path)

        # Empty folders are not included when the package is installed
        (case_path / "constant" / "triSurface").mkdir(parents=True, exist_ok=True)

        self._select_time_control_files(case_path)
        self._write_control_dict(case_path)
        self._write_block_mesh_dict(case_path)
        self._write_snappy_hex_mesh_dict(case_path)
        self._write_boundary_conditions(case_path)
        self._write_actuator_line(case_path)

    @staticmethod
    def run_foam_case(path: Path | str, nr_processes: int | None = None) -> None:
        '''
        Runs a case written by `write_foam_case`, using the run script in the case folder, and waits
        until it is finished. OpenFOAM must be loaded in the terminal. The solver runs in parallel
        if `nr_processes` is larger than one. The default is the number of physical cores.
        '''
        arguments = [] if nr_processes is None else [str(nr_processes)]

        result = subprocess.run(["bash", "run.sh", *arguments], cwd=path)

        if result.returncode != 0:
            raise RuntimeError(f"The OpenFOAM case in {path} failed. See the log files in the case folder.")

    @staticmethod
    def read_results(path: Path | str) -> ActuatorLineResults:
        '''
        Reads the results from the actuator line model in a case that has been run
        '''
        return ActuatorLineResults.from_case(path)

    def _write_actuator_line(self, case_path: Path) -> None:
        '''
        Writes the actuator line setup to the file read by the OpenFOAM interface. Without an
        actuator line model, the actuator line is removed from the case.
        '''
        if self.actuator_line is not None:
            self.actuator_line.to_json_file(case_path / "system" / "stormbird_actuator_line.json")

            return

        (case_path / "system" / "fvOptions").unlink()

        control_dict_path = case_path / "system" / "controlDict"

        control_dict = read_foam_file(control_dict_path)
        control_dict.pop("libs", None)

        write_foam_file(control_dict_path, control_dict)

    @staticmethod
    def _convert_to_lf_line_endings(case_path: Path) -> None:
        '''
        Converts CRLF line endings to LF in all text files in the case folder. The base folder can
        get CRLF line endings when the package is built on Windows, which makes both bash and
        OpenFOAM fail. Files containing null bytes are treated as binary, and are not changed.
        '''
        for file_path in case_path.rglob("*"):
            if not file_path.is_file():
                continue

            content = file_path.read_bytes()

            if b"\r\n" in content and b"\0" not in content:
                file_path.write_bytes(content.replace(b"\r\n", b"\n"))

    def _select_time_control_files(self, case_path: Path) -> None:
        '''
        The base folder contains a steady and a dynamic version of some files in the system folder,
        marked with the suffixes `.steady` and `.dynamic`. The version matching the time control is
        renamed to the normal file name, while the other version is removed.
        '''
        selected = "steady" if isinstance(self.time_control, SteadyTimeControl) else "dynamic"

        for file_path in (case_path / "system").iterdir():
            if file_path.suffix == "." + selected:
                file_path.rename(file_path.with_suffix(""))
            elif file_path.suffix in (".steady", ".dynamic"):
                file_path.unlink()

    def _write_control_dict(self, case_path: Path) -> None:
        '''
        Sets the duration of the simulation. Results are written 10 times during the simulation, and
        only the latest results are kept, as set by purgeWrite in the base folder.
        '''
        file_path = case_path / "system" / "controlDict"

        control_dict = read_foam_file(file_path)

        if isinstance(self.time_control, SteadyTimeControl):
            control_dict["endTime"] = self.time_control.max_iterations
            control_dict["deltaT"] = 1
            control_dict["writeInterval"] = self.time_control.max_iterations / NR_WRITES
        else:
            control_dict["endTime"] = self.time_control.end_time
            control_dict["deltaT"] = self.time_control.time_step
            control_dict["maxDeltaT"] = self.time_control.time_step
            control_dict["maxCo"] = self.time_control.max_courant_number
            control_dict["writeInterval"] = self.time_control.end_time / NR_WRITES

        write_foam_file(file_path, control_dict)

    def _write_block_mesh_dict(self, case_path: Path) -> None:
        '''
        Sets the domain size and the number of cells in the background mesh. The vertices are
        written in the same order as in the base folder, so that the boundary patches there can be
        kept. With the wind tunnel boundary conditions, the patches are replaced by the inlet, the
        outlet and the tunnel walls.
        '''
        file_path = case_path / "system" / "blockMeshDict"

        block_mesh_dict = read_foam_file(file_path)

        x0, y0, z0 = self.grid.start_point.as_tuple()
        x1, y1, z1 = self.grid.end_point.as_tuple()

        nx, ny, nz = self.grid.background_mesh_shape(self.representative_length())

        block_mesh_dict["vertices"] = [
            (x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
            (x0, y0, z1), (x1, y0, z1), (x1, y1, z1), (x0, y1, z1),
        ]
        block_mesh_dict["blocks"] = [
            f"hex (0 1 2 3 4 5 6 7) ({nx} {ny} {nz}) simpleGrading (1 1 1)"
        ]

        if self.boundary_condition_type == BoundaryConditionType.WindTunnel:
            block_mesh_dict["boundary"] = _wind_tunnel_boundary_string()

        write_foam_file(file_path, block_mesh_dict)

    def named_geometries(self) -> list[tuple[str, GeometryBuilder, bool]]:
        '''
        The geometries in the case, with the name used for each of them in OpenFOAM and whether
        they are slip geometries. The name is also the name of the resulting patch in the mesh.
        '''
        return [
            (f"geometry_{index}", geometry, False) for index, geometry in enumerate(self.geometries)
        ] + [
            (f"slip_geometry_{index}", geometry, True) for index, geometry in enumerate(self.slip_geometries)
        ]

    def _write_snappy_hex_mesh_dict(self, case_path: Path) -> None:
        '''
        Adds the geometries as snapped surfaces, and a cylindrical refinement zone around each wing
        in the actuator line model. Layers are not used.
        '''
        file_path = case_path / "system" / "snappyHexMeshDict"

        snappy_dict = read_foam_file(file_path)

        geometry: FoamDict = {}
        refinement_surfaces: FoamDict = {}
        refinement_regions: FoamDict = {}

        if self.grid.geometry_cells_per_representative_length is None:
            surface_level = 0
        else:
            surface_level = self.grid.refinement_level(self.grid.geometry_cells_per_representative_length)

        for name, geometry_builder, is_slip in self.named_geometries():
            key, entry = self._snappy_geometry_entry(name, geometry_builder, case_path)
            geometry[key] = entry

            refinement_surfaces[name] = {
                "level": (surface_level, surface_level),
                "patchInfo": {"type": "patch" if is_slip else "wall"},
            }

        if self.actuator_line is not None and self.grid.actuator_line_refinement is not None:
            refinement = self.grid.actuator_line_refinement
            level = self.grid.refinement_level(refinement.cells_per_representative_length)

            gaussian = self.actuator_line.projection_settings.projection_function
            width_factor = max(gaussian.chord_factor, gaussian.thickness_factor)

            # snappyHexMesh only refines the cells with their center inside the region. The cylinder
            # is therefore extended by half the diagonal of a background cell, so that all cells that
            # overlap the refinement zone are refined, also when the zone is smaller than the cells.
            background_cell_length = (
                self.representative_length() / self.grid.background_mesh_cells_per_representative_length
            )
            margin = 0.5 * math.sqrt(3.0) * background_cell_length

            for index, wing_builder in enumerate(self.actuator_line.line_force_model.wing_builders):
                name = f"actuator_line_{index}"

                chord_lengths = [chord_vector.length() for chord_vector in wing_builder.chord_vectors]
                mean_chord = sum(chord_lengths) / len(chord_lengths)

                start_point = wing_builder.section_points[0]
                end_point = wing_builder.section_points[-1]

                axis = (end_point - start_point) * (1.0 / (end_point - start_point).length())

                geometry[name] = {
                    "type": "searchableCylinder",
                    "point1": (start_point - axis * margin).as_tuple(),
                    "point2": (end_point + axis * margin).as_tuple(),
                    "radius": 0.5 * refinement.padding * mean_chord * width_factor + margin,
                }

                refinement_regions[name] = {"mode": "inside", "levels": [("1e15", level)]}

        snappy_dict["castellatedMesh"] = True
        snappy_dict["snap"] = True
        snappy_dict["addLayers"] = False
        snappy_dict["geometry"] = geometry

        castellated_controls = _sub_dict(snappy_dict, "castellatedMeshControls")

        castellated_controls["refinementSurfaces"] = refinement_surfaces
        castellated_controls["refinementRegions"] = refinement_regions
        castellated_controls["locationInMesh"] = self._location_in_mesh()

        write_foam_file(file_path, snappy_dict)

    @staticmethod
    def _snappy_geometry_entry(
        name: str,
        geometry_builder: GeometryBuilder,
        case_path: Path
    ) -> tuple[str, FoamDict]:
        '''
        Returns the key and the content of the entry in the geometry dictionary of snappyHexMesh.
        Triangle meshes are copied to constant/triSurface.
        '''
        shape = geometry_builder.geometry

        if isinstance(shape, TriangleMeshBuilder):
            source = Path(shape.file_path)
            file_name = name + source.suffix

            shutil.copyfile(source, case_path / "constant" / "triSurface" / file_name)

            return file_name, {"type": "triSurfaceMesh", "name": name}

        if isinstance(shape, Sphere):
            return name, {
                "type": "searchableSphere",
                "centre": shape.center.as_tuple(),
                "radius": shape.radius,
            }

        if isinstance(shape, Cuboid):
            return name, {
                "type": "searchableBox",
                "min": (shape.center - shape.half_extents).as_tuple(),
                "max": (shape.center + shape.half_extents).as_tuple(),
            }

        if isinstance(shape, Disk):
            # Extruded from the center along the normal, as in Stormflow
            return name, {
                "type": "searchableCylinder",
                "point1": shape.center.as_tuple(),
                "point2": (shape.center + shape.normal.normalize() * shape.thickness).as_tuple(),
                "radius": shape.radius,
            }

        raise ValueError(f"{type(shape).__name__} geometries are not supported in OpenFOAM cases")

    def _location_in_mesh(self) -> tuple[float, float, float]:
        '''
        A point inside the first cell of the background mesh, used by snappyHexMesh to decide which
        part of the mesh to keep. The point is placed off the cell center, so that it does not end
        up on a face after refinement.
        '''
        shape = self.grid.background_mesh_shape(self.representative_length())

        start = self.grid.start_point.as_tuple()
        end = self.grid.end_point.as_tuple()

        x, y, z = (
            start[axis] + 0.437 * (end[axis] - start[axis]) / shape[axis] for axis in range(3)
        )

        return (x, y, z)

    def apparent_wind_at_height(self, height: float) -> SpatialVector:
        '''
        The steady apparent wind velocity at the given height above the water plane
        '''
        location = SpatialVector(x=0.0, y=0.0, z=self.wind_environment.water_plane_height + height)

        return self.wind_environment.steady_apparent_wind_velocity_vector_at_location(
            self.wind_condition,
            location,
            self.linear_velocity
        )

    def turbulence_reference_velocity(self) -> float:
        '''
        The velocity used to compute the turbulence in the inflow. This is the true wind velocity at
        10 m height, or the magnitude of the linear velocity if there is no true wind.
        '''
        velocity = self.wind_condition.steady_true_wind_velocity_at_height(TURBULENCE_REFERENCE_HEIGHT)

        if velocity == 0.0:
            velocity = self.linear_velocity.length()

        if velocity == 0.0:
            raise ValueError("Either the true wind or the linear velocity must be non-zero")

        return velocity

    def _write_boundary_conditions(self, case_path: Path) -> None:
        '''
        Sets the apparent wind and turbulence in the inflow, and adds boundary conditions for the
        geometries. No-slip geometries use wall functions for the turbulence. With the wind tunnel
        boundary conditions, the patches of the base folder are replaced by the wind tunnel patches.
        '''
        reference_velocity = self.turbulence_reference_velocity()

        transport_properties = read_foam_file(case_path / "constant" / "transportProperties")
        viscosity = float(str(transport_properties["nu"]).split()[-1])

        k = self.inlet_turbulence.k(reference_velocity)
        omega = self.inlet_turbulence.omega(reference_velocity, viscosity)

        no_slip_conditions: dict[str, FoamDict] = {
            "U": {"type": "noSlip"},
            "p": {"type": "zeroGradient"},
            "k": {"type": "kqRWallFunction", "value": "$internalField"},
            "omega": {"type": "omegaWallFunction", "value": "$internalField"},
            "nut": {"type": "nutkWallFunction", "value": "uniform 0"},
        }

        for field_name, no_slip_condition in no_slip_conditions.items():
            file_path = case_path / "0" / field_name

            field = read_foam_file(file_path)
            boundary_field = _sub_dict(field, "boundaryField")

            if self.boundary_condition_type == BoundaryConditionType.WindTunnel:
                _replace_with_wind_tunnel_patches(boundary_field)

            for name, _, is_slip in self.named_geometries():
                boundary_field[name] = {"type": "slip"} if is_slip else dict(no_slip_condition)

            if field_name == "U":
                initial_velocity = self.apparent_wind_at_height(TURBULENCE_REFERENCE_HEIGHT)

                field["internalField"] = "uniform " + _vector_string(initial_velocity)

                apparent_wind = _sub_dict(
                    boundary_field, self.inlet_patch_name(), "codeContext", "apparentWind"
                )
                apparent_wind["values"] = self._apparent_wind_table()
            elif field_name == "k":
                field["internalField"] = f"uniform {k}"
            elif field_name == "omega":
                field["internalField"] = f"uniform {omega}"

            write_foam_file(file_path, field)

    def _apparent_wind_profile(self) -> list[tuple[float, tuple[float, float, float]]]:
        '''
        The apparent wind velocity as a function of the vertical coordinate, covering the domain
        '''
        z_coordinates = np.linspace(self.grid.start_point.z, self.grid.end_point.z, NR_WIND_TABLE_HEIGHTS)

        profile = []

        for z in z_coordinates:
            velocity = self.wind_environment.steady_apparent_wind_velocity_vector_at_location(
                self.wind_condition,
                SpatialVector(x=0.0, y=0.0, z=float(z)),
                self.linear_velocity
            )

            profile.append((float(z), velocity.as_tuple()))

        return profile

    def _apparent_wind_table(self) -> list[FoamValue]:
        '''
        The apparent wind profile as a table in an OpenFOAM file
        '''
        return [(z, velocity) for z, velocity in self._apparent_wind_profile()]


def _sub_dict(data: FoamDict, *keys: str) -> FoamDict:
    '''
    Returns the sub-dictionary found by following the keys, and raises an error if it is missing
    '''
    for key in keys:
        value = data.get(key)

        if not isinstance(value, dict):
            raise ValueError(f"Expected a dictionary named '{key}' in the OpenFOAM file")

        data = value

    return data

def _wind_tunnel_boundary_string() -> str:
    '''
    The boundary list in the blockMeshDict for the wind tunnel patches
    '''
    lines = ["("]

    for name, faces in WIND_TUNNEL_PATCH_FACES.items():
        lines += [f"\t{name}", "\t{", "\t\ttype patch;", "\t\tfaces", "\t\t("]
        lines += [f"\t\t\t{face}" for face in faces]
        lines += ["\t\t);", "\t}", ""]

    lines.append(")")

    return "\n".join(lines)

def _replace_with_wind_tunnel_patches(boundary_field: FoamDict) -> None:
    '''
    Replaces the patches of the base folder with the wind tunnel patches in the boundary field of a
    field file. The conditions already in the base folder are reused: the inlet gets the conditions
    of the vertical sides, the outlet the conditions of the open top, and the walls the slip
    conditions of the ground.
    '''
    sources = {
        name: _sub_dict(boundary_field, source_name)
        for name, source_name in WIND_TUNNEL_PATCH_SOURCES.items()
    }

    for source_name in WIND_TUNNEL_PATCH_SOURCES.values():
        boundary_field.pop(source_name)

    for name, condition in sources.items():
        boundary_field[name] = condition

def _vector_string(vector: SpatialVector) -> str:
    x, y, z = vector.as_tuple()

    return f"({x} {y} {z})"
