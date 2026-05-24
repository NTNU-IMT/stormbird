
use super::*;

use std::fs;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

impl TriangleMesh {
    pub fn new_from_obj_file(path: &str) -> Self {
        let file_contents = fs::read_to_string(path).expect("Failed to read OBJ file");

        let mut vertices: Vec<SpatialVector> = Vec::new();
        let mut triangles: Vec<[SpatialVector; 3]> = Vec::new();

        for line in file_contents.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    let x: Float = parts[1].parse().unwrap();
                    let y: Float = parts[2].parse().unwrap();
                    let z: Float = parts[3].parse().unwrap();
                    vertices.push(SpatialVector::from([x, y, z]));
                }
                "f" => {
                    // Parse all vertex indices for this face
                    let face_indices: Vec<usize> = parts[1..]
                        .iter()
                        .map(|part| {
                            let v: usize = part.split('/').next().unwrap().parse().unwrap();
                            v - 1  // Convert to 0-indexed
                        })
                        .collect();

                    if face_indices.len() == 3 {
                        // Triangle - add directly
                        triangles.push([
                            vertices[face_indices[0]],
                            vertices[face_indices[1]],
                            vertices[face_indices[2]],
                        ]);
                    } else if face_indices.len() > 3 {
                        // Polygon with more than 3 vertices - triangulate using center point fan

                        // Calculate center of polygon (average of all vertices)
                        let center = face_indices
                            .iter()
                            .map(|&i| vertices[i])
                            .fold(SpatialVector::default(), |acc, v| acc + v)
                            / (face_indices.len() as Float);

                        // Create fan triangles from center to each edge
                        for i in 0..face_indices.len() {
                            let next_i = (i + 1) % face_indices.len();
                            triangles.push([
                                center,
                                vertices[face_indices[i]],
                                vertices[face_indices[next_i]],
                            ]);
                        }
                    }
                }
                _ => {}
            }
        }

        Self {
            triangles
        }
    }
}