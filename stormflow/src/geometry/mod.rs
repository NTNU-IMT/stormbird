
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

use serde::{Serialize, Deserialize};

pub mod triangle_mesh;
pub mod analytical_shapes;

use crate::grid::Grid;

use rayon::prelude::*;

use triangle_mesh::{
    TriangleMesh,
    TriangleMeshBuilder
};
use analytical_shapes::{
    Sphere,
    Cuboid,
    Disk
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeometryBuilder {
    Sphere(Sphere),
    Cuboid(Cuboid),
    Disk(Disk),
    TriangleMesh(TriangleMeshBuilder)
}

impl GeometryBuilder {
    pub fn build(&self) -> Geometry {
        match self {
            Self::Cuboid(cuboid) => Geometry::Cuboid(cuboid.clone()),
            Self::Sphere(sphere) => Geometry::Sphere(sphere.clone()),
            Self::Disk(disk) => Geometry::Disk(disk.clone()),
            Self::TriangleMesh(tri_mesh_builder) => Geometry::TriangleMesh(tri_mesh_builder.build())
        }
    }
}

#[derive(Debug, Clone)]
pub enum Geometry {
    Sphere(Sphere),
    Cuboid(Cuboid),
    Disk(Disk),
    TriangleMesh(TriangleMesh)
}

impl Geometry {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        match self {
            Self::Sphere(sphere) => sphere.signed_distance(point),
            Self::Cuboid(cuboid) => cuboid.signed_distance(point),
            Self::Disk(disk) => disk.signed_distance(point),
            Self::TriangleMesh(triangle_mesh) => triangle_mesh.signed_distance(point)
        }
    }

    /// Computes the union of the signed distance functions in geometries
    pub fn signed_distance_function_union(geometries: &[Geometry], point: SpatialVector) -> Float {
        let mut value = Float::MAX;
        
        for geometry in geometries {
            let local_value = geometry.signed_distance(point);
            
            if local_value < value {
                value = local_value;
            }
        }
        
        value
    }

    pub fn signed_distance_function_on_extended_grid(geometries: &[Geometry], grid: &Grid) -> Vec<Float> {
        let nr_extended_cells = grid.nr_extended_cells();

        (0..nr_extended_cells).into_par_iter()
            .map(|i_flat_extended| {
                let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
                
                let cell_center = grid.cell_center_extended(extended_indices);

                Geometry::signed_distance_function_union(
                    geometries, 
                    cell_center
                )
            }).collect()
    }

    /// Computes the surface normal direction at each grid point using finite differences
    /// on the signed distance function union. The normal is computed as the normalized
    /// gradient of the SDF: n = ∇φ / |∇φ|
    pub fn geometry_normals_on_extended_grid(
        geometries: &[Geometry], 
        grid: &Grid, 
        delta_factor: Float
    ) -> Vec<SpatialVector> {
        let delta = delta_factor * grid.cell_length;
        
        let nr_extended_cells = grid.nr_extended_cells();

        (0..nr_extended_cells).into_par_iter()
            .map(|i_flat_extended| {
                let extended_indices = grid.extended_indices_from_flat_index(i_flat_extended);
                let cell_center = grid.cell_center_extended(extended_indices);

                // Compute gradient using central finite differences
                let dx = Geometry::signed_distance_function_union(
                    geometries,
                    cell_center + SpatialVector::new(delta[0], 0.0, 0.0)
                ) - Geometry::signed_distance_function_union(
                    geometries,
                    cell_center - SpatialVector::new(delta[0], 0.0, 0.0)
                );

                let dy = Geometry::signed_distance_function_union(
                    geometries,
                    cell_center + SpatialVector::new(0.0, delta[1], 0.0)
                ) - Geometry::signed_distance_function_union(
                    geometries,
                    cell_center - SpatialVector::new(0.0, delta[1], 0.0)
                );

                let dz = Geometry::signed_distance_function_union(
                    geometries,
                    cell_center + SpatialVector::new(0.0, 0.0, delta[2])
                ) - Geometry::signed_distance_function_union(
                    geometries,
                    cell_center - SpatialVector::new(0.0, 0.0, delta[2])
                );

                let gradient = SpatialVector::new(
                    dx / (2.0 * delta[0]),
                    dy / (2.0 * delta[1]),
                    dz / (2.0 * delta[2])
                );

                // Normalize to get unit normal (handle zero gradient case)
                let length = gradient.length();
                if length > 1e-12 {
                    gradient / length
                } else {
                    SpatialVector::default()
                }
            }).collect()
    }

    /// Blending function that goes between 0 and 1 from -epsilon to epsilon
    pub fn blending_function(distance: Float, epsilon: Float) -> Float {
        if distance > epsilon {
            1.0
        } else if distance < -epsilon {
            0.0
        } else {
            0.5 * (1.0 + distance/epsilon + (PI * distance / epsilon).sin() / PI)
        }
    }
}





