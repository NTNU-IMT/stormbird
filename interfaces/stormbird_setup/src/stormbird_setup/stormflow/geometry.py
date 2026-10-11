"""
Copyright (C) 2024, NTNU
Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)
"""

from typing import Any

from pydantic import model_serializer, model_validator

from ..base_model import StormbirdSetupBaseModel
from ..spatial_vector import SpatialVector


class Sphere(StormbirdSetupBaseModel):
    center: SpatialVector
    radius: float


class Cuboid(StormbirdSetupBaseModel):
    center: SpatialVector
    half_extents: SpatialVector
    """Half the size of the cuboid along each axis, i.e. (hx, hy, hz)."""


class Disk(StormbirdSetupBaseModel):
    center: SpatialVector
    normal: SpatialVector
    """Unit normal defining the plane of the disk."""
    radius: float
    thickness: float = 0.0
    """
    Thickness of the disk, extruded from `center` along `normal` (i.e. spanning [0, thickness]).
    Zero thickness recovers the flat disk.
    """
    fillet_radius: float = 0.0
    """Fillet (rounding) radius applied to the edges created by the extrusion."""


class TriangleMeshBuilder(StormbirdSetupBaseModel):
    file_path: str
    """Path to the .obj file that defines the triangle mesh."""


# The name of each variant in the Rust `GeometryBuilder` enum, mapped to its Python class. Serde
# serializes this enum in its default, externally-tagged form (e.g. `{"Sphere": {...}}`).
_GEOMETRY_VARIANTS: dict[str, type[StormbirdSetupBaseModel]] = {
    "Sphere": Sphere,
    "Cuboid": Cuboid,
    "Disk": Disk,
    "TriangleMesh": TriangleMeshBuilder,
}


class GeometryBuilder(StormbirdSetupBaseModel):
    """
    Wrapper representing the Rust `GeometryBuilder` enum. Serializes into the externally-tagged form
    expected by serde, e.g. `{"Sphere": {"center": {...}, "radius": 1.0}}`.
    """

    geometry: Sphere | Cuboid | Disk | TriangleMeshBuilder

    @classmethod
    def new_sphere(cls, center: SpatialVector, radius: float) -> "GeometryBuilder":
        return cls(geometry=Sphere(center=center, radius=radius))

    @classmethod
    def new_cuboid(cls, center: SpatialVector, half_extents: SpatialVector) -> "GeometryBuilder":
        return cls(geometry=Cuboid(center=center, half_extents=half_extents))

    @classmethod
    def new_disk(
        cls,
        center: SpatialVector,
        normal: SpatialVector,
        radius: float,
        thickness: float = 0.0,
        fillet_radius: float = 0.0,
    ) -> "GeometryBuilder":
        return cls(
            geometry=Disk(
                center=center,
                normal=normal,
                radius=radius,
                thickness=thickness,
                fillet_radius=fillet_radius,
            )
        )

    @classmethod
    def new_triangle_mesh(cls, file_path: str) -> "GeometryBuilder":
        return cls(geometry=TriangleMeshBuilder(file_path=file_path))

    @model_validator(mode="before")
    @classmethod
    def _deserialize_from_rust_enum(cls, data: Any) -> Any:
        # Already in Python/Pydantic form
        if isinstance(data, dict) and "geometry" in data:
            return data

        # Rust externally-tagged enum form, e.g. {"Sphere": {...}}
        if isinstance(data, dict) and len(data) == 1:
            (tag, inner), = data.items()

            if tag in _GEOMETRY_VARIANTS:
                return {"geometry": _GEOMETRY_VARIANTS[tag].model_validate(inner)}

        return data

    @model_serializer
    def _serialize_to_rust_enum(self) -> dict[str, Any]:
        for tag, variant in _GEOMETRY_VARIANTS.items():
            if isinstance(self.geometry, variant):
                return {tag: self.geometry.model_dump(exclude_none=True, mode="json")}

        raise ValueError(f"Unknown geometry variant: {type(self.geometry)}")
