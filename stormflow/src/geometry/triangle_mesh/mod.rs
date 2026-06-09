
use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

pub mod io;
pub mod kernels;

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangleMeshBuilder {
    pub file_path: String
}

impl TriangleMeshBuilder {
    pub fn build(&self) -> TriangleMesh {
        TriangleMesh::new_from_obj_file(&self.file_path)
    }
}

#[derive(Debug, Clone)]
pub struct TriangleMesh {
    pub triangles: Vec<[SpatialVector; 3]>
}

impl TriangleMesh {
    pub fn signed_distance(&self, point: SpatialVector) -> Float {
        let mut min_distance = Float::MAX;

        let mut solid_angle_sum = 0.0;
        
        for triangle in &self.triangles {
            let current_distance = kernels::distance_to_triangle(point, *triangle);

            if current_distance < min_distance {
                min_distance = current_distance;
            }

            solid_angle_sum += kernels::solid_angle(point, *triangle)
        }

        let winding_number = solid_angle_sum / (4.0 * PI);

        let sign = if winding_number > 0.5 {
            -1.0
        } else {
            1.0
        };

        min_distance * sign
    }
}