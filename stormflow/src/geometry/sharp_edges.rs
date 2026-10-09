//! Extraction of the sharp convex edges of the input geometries, used by the wall model to apply
//! the data immersion close to the edges (see `velocity_solver::sharp_edges`).
//!
//! The edges are taken directly from the geometry definitions: the twelve edges of a cuboid, the
//! rims of a disk, and the feature edges of a triangle mesh, which are the edges where the two
//! neighboring triangles meet at a convex angle larger than a threshold. Spheres have no edges.

use std::collections::HashMap;

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;
use stormath::consts::PI;

use super::Geometry;
use super::analytical_shapes::{Cuboid, Disk};
use super::triangle_mesh::TriangleMesh;

#[derive(Debug, Clone, Copy)]
/// A straight segment of a sharp convex edge
pub struct SharpEdge {
    pub start: SpatialVector,
    pub end: SpatialVector,
    /// Unit vector pointing out of the geometry, along the bisector of the two surface normals
    /// that meet at the edge. Used to check whether the edge is covered by another geometry.
    pub outward: SpatialVector,
}

impl SharpEdge {
    /// The shortest distance from `point` to the segment
    pub fn distance(&self, point: SpatialVector) -> Float {
        let direction = self.end - self.start;
        let length_squared = direction.length_squared();

        let t = if length_squared > 0.0 {
            ((point - self.start).dot(direction) / length_squared).clamp(0.0, 1.0)
        } else {
            0.0
        };

        (point - (self.start + t * direction)).length()
    }

    /// Splits the segment into equally long segments that are at most `max_length` long
    pub fn subdivided(&self, max_length: Float) -> Vec<SharpEdge> {
        let length = (self.end - self.start).length();
        let nr_segments = ((length / max_length).ceil() as usize).max(1);

        (0..nr_segments).map(|i| {
            let t0 = i as Float / nr_segments as Float;
            let t1 = (i + 1) as Float / nr_segments as Float;

            SharpEdge {
                start: self.start + t0 * (self.end - self.start),
                end: self.start + t1 * (self.end - self.start),
                outward: self.outward,
            }
        }).collect()
    }
}

impl Geometry {
    /// The sharp convex edges of the geometry.
    ///
    /// - `min_angle` is the smallest angle, in radians, between the two surface normals at an
    ///   edge of a triangle mesh for it to count as sharp. Smaller angles are treated as a
    ///   discretization of a smooth surface.
    /// - `max_unresolved_radius` is the largest rounding (fillet) radius of the analytical shapes
    ///   for which the edge still counts as sharp, as a rounding smaller than about a cell can not
    ///   be resolved by the grid anyway.
    /// - `max_arc_segment_length` is the largest length of the straight segments used to represent
    ///   curved edges.
    pub fn sharp_edges(
        &self,
        min_angle: Float,
        max_unresolved_radius: Float,
        max_arc_segment_length: Float,
    ) -> Vec<SharpEdge> {
        match self {
            Self::Sphere(_) => Vec::new(),
            Self::Cuboid(cuboid) => cuboid_edges(cuboid),
            Self::Disk(disk) => disk_edges(disk, max_unresolved_radius, max_arc_segment_length),
            Self::TriangleMesh(mesh) => triangle_mesh_edges(mesh, min_angle),
        }
    }
}

/// The twelve edges of the (axis-aligned) cuboid
fn cuboid_edges(cuboid: &Cuboid) -> Vec<SharpEdge> {
    let mut edges = Vec::with_capacity(12);

    for axis in 0..3 {
        let other_axes = [(axis + 1) % 3, (axis + 2) % 3];

        for sign_0 in [-1.0, 1.0] {
            for sign_1 in [-1.0, 1.0] {
                let mut offset = SpatialVector::default();
                offset[other_axes[0]] = sign_0 * cuboid.half_extents[other_axes[0]];
                offset[other_axes[1]] = sign_1 * cuboid.half_extents[other_axes[1]];

                let mut outward = SpatialVector::default();
                outward[other_axes[0]] = sign_0;
                outward[other_axes[1]] = sign_1;

                let mut along = SpatialVector::default();
                along[axis] = cuboid.half_extents[axis];

                edges.push(SharpEdge {
                    start: cuboid.center + offset - along,
                    end: cuboid.center + offset + along,
                    outward: outward.normalize(),
                });
            }
        }
    }

    edges
}

/// The rims of the disk: one for a flat disk, and two (at both ends of the extrusion) for a disk
/// with a thickness. A disk with a fillet radius larger than `max_unresolved_radius` has no sharp
/// edges.
fn disk_edges(disk: &Disk, max_unresolved_radius: Float, max_segment_length: Float) -> Vec<SharpEdge> {
    if disk.fillet_radius > max_unresolved_radius || disk.radius <= 0.0 {
        return Vec::new();
    }

    let normal = disk.normal.normalize();

    // Two unit vectors in the plane of the disk
    let helper = if normal[0].abs() < 0.9 { SpatialVector::unit_x() } else { SpatialVector::unit_y() };
    let in_plane_0 = helper.cross(normal).normalize();
    let in_plane_1 = normal.cross(in_plane_0);

    // (height along the normal, component of the outward direction along the normal)
    let rims: Vec<(Float, Float)> = if disk.thickness > 0.0 {
        vec![(0.0, -1.0), (disk.thickness, 1.0)]
    } else {
        vec![(0.0, 0.0)]
    };

    let circumference = 2.0 * PI * disk.radius;
    let nr_segments = ((circumference / max_segment_length).ceil() as usize).max(32);

    let mut edges = Vec::with_capacity(rims.len() * nr_segments);

    for (height, normal_component) in rims {
        let point_at = |angle: Float| {
            disk.center + height * normal +
            disk.radius * (angle.cos() * in_plane_0 + angle.sin() * in_plane_1)
        };

        for i in 0..nr_segments {
            let angle_0 = 2.0 * PI * i as Float / nr_segments as Float;
            let angle_1 = 2.0 * PI * (i + 1) as Float / nr_segments as Float;
            let angle_mid = 0.5 * (angle_0 + angle_1);

            let radial = angle_mid.cos() * in_plane_0 + angle_mid.sin() * in_plane_1;

            edges.push(SharpEdge {
                start: point_at(angle_0),
                end: point_at(angle_1),
                outward: (radial + normal_component * normal).normalize(),
            });
        }
    }

    edges
}

/// Key for identifying vertices that are shared between triangles. The vertices of a triangle
/// mesh are read from indexed OBJ faces, so shared vertices are bit-wise identical. Adding zero
/// turns `-0.0` into `0.0`, so that the two get the same key.
fn vertex_key(vertex: SpatialVector) -> [u64; 3] {
    std::array::from_fn(|i| ((vertex[i] + 0.0) as f64).to_bits())
}

/// The feature edges of a triangle mesh: the edges shared by two triangles where the angle
/// between the two outward normals is larger than `min_angle`, and the surface is convex.
///
/// The mesh is assumed to be watertight and consistently oriented. The orientation (whether the
/// triangle normals point in or out) is determined from the sign of the enclosed volume. Edges
/// that do not have exactly two neighboring triangles are skipped, with a warning.
fn triangle_mesh_edges(mesh: &TriangleMesh, min_angle: Float) -> Vec<SharpEdge> {
    let signed_volume: Float = mesh.triangles.iter().map(|[a, b, c]| {
        a.dot(b.cross(*c))
    }).sum();

    let orientation = if signed_volume < 0.0 { -1.0 } else { 1.0 };

    let normals: Vec<SpatialVector> = mesh.triangles.iter().map(|[a, b, c]| {
        orientation * (*b - *a).cross(*c - *a).normalize()
    }).collect();

    // Edge key -> (the two vertices of the edge, the neighboring triangles and the index of the
    // vertex opposite to the edge in each of them)
    let mut edge_map: HashMap<([u64; 3], [u64; 3]), (SpatialVector, SpatialVector, Vec<(usize, usize)>)> =
        HashMap::new();

    for (triangle_index, triangle) in mesh.triangles.iter().enumerate() {
        for local_edge in 0..3 {
            let p = triangle[local_edge];
            let q = triangle[(local_edge + 1) % 3];
            let opposite = (local_edge + 2) % 3;

            let (key_p, key_q) = (vertex_key(p), vertex_key(q));

            if key_p == key_q {
                continue; // Degenerate edge
            }

            let (key, start, end) = if key_p < key_q {
                ((key_p, key_q), p, q)
            } else {
                ((key_q, key_p), q, p)
            };

            edge_map.entry(key)
                .or_insert_with(|| (start, end, Vec::new()))
                .2.push((triangle_index, opposite));
        }
    }

    let cos_min_angle = min_angle.cos();

    let mut nr_non_manifold_edges = 0;

    let mut edges: Vec<SharpEdge> = Vec::new();

    for (start, end, neighbors) in edge_map.values() {
        if neighbors.len() != 2 {
            nr_non_manifold_edges += 1;
            continue;
        }

        let (triangle_a, _) = neighbors[0];
        let (triangle_b, opposite_b) = neighbors[1];

        let normal_a = normals[triangle_a];
        let normal_b = normals[triangle_b];

        // Degenerate triangles have no normal, and can not define a sharp edge
        if normal_a.length_squared() == 0.0 || normal_b.length_squared() == 0.0 {
            continue;
        }

        if normal_a.dot(normal_b) > cos_min_angle {
            continue;
        }

        // Convex if the opposite vertex of the second triangle is behind the plane of the first
        let opposite_vertex_b = mesh.triangles[triangle_b][opposite_b];
        let edge_length = (*end - *start).length();

        if normal_a.dot(opposite_vertex_b - *start) >= -1e-6 * edge_length {
            continue;
        }

        let bisector = normal_a + normal_b;

        // A knife edge, where the two normals are opposite: the outward direction is then along
        // the first triangle, away from its opposite vertex
        let outward = if bisector.length() > 1e-6 {
            bisector.normalize()
        } else {
            let opposite_vertex_a = mesh.triangles[triangle_a][neighbors[0].1];
            let away = *start - opposite_vertex_a;
            let along_edge = (*end - *start).normalize();

            (away - away.dot(along_edge) * along_edge).normalize()
        };

        edges.push(SharpEdge {
            start: *start,
            end: *end,
            outward,
        });
    }

    if nr_non_manifold_edges > 0 {
        println!(
            "Warning: {} edges of a triangle mesh do not have exactly two neighboring triangles, \
             and are not checked for sharpness. Is the mesh watertight?",
            nr_non_manifold_edges
        );
    }

    // The hash map has no defined order, so the edges are sorted to make the result deterministic
    edges.sort_by(|a, b| {
        let key_a = (vertex_key(a.start), vertex_key(a.end));
        let key_b = (vertex_key(b.start), vertex_key(b.end));

        key_a.cmp(&key_b)
    });

    edges
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit cube as a triangle mesh, with each square side split into two triangles, and
    /// optionally with all triangles flipped (normals pointing inwards).
    fn cube_mesh(flip: bool) -> TriangleMesh {
        let corner = |i: usize| SpatialVector([
            (i & 1) as Float, ((i >> 1) & 1) as Float, ((i >> 2) & 1) as Float
        ]);

        // The sides as quads, counter-clockwise seen from the outside
        let quads = [
            [0, 2, 3, 1], // z = 0
            [4, 5, 7, 6], // z = 1
            [0, 1, 5, 4], // y = 0
            [2, 6, 7, 3], // y = 1
            [0, 4, 6, 2], // x = 0
            [1, 3, 7, 5], // x = 1
        ];

        let mut triangles = Vec::new();

        for quad in quads {
            let [a, b, c, d] = quad.map(corner);

            for mut triangle in [[a, b, c], [a, c, d]] {
                if flip {
                    triangle.swap(1, 2);
                }

                triangles.push(triangle);
            }
        }

        TriangleMesh { triangles }
    }

    fn assert_cube_edges(edges: &[SharpEdge]) {
        assert_eq!(edges.len(), 12);

        let center = SpatialVector([0.5, 0.5, 0.5]);

        for edge in edges {
            assert!(((edge.end - edge.start).length() - 1.0).abs() < 1e-5);

            // The outward direction points away from the center, at 45 degrees to the sides
            let midpoint = 0.5 * (edge.start + edge.end);
            let expected = (midpoint - center).normalize();

            assert!((edge.outward - expected).length() < 1e-5, "{:?}", edge);
        }
    }

    #[test]
    fn cube_mesh_has_twelve_sharp_edges_independent_of_orientation() {
        let min_angle = 30.0_f64.to_radians() as Float;

        for flip in [false, true] {
            let edges = Geometry::TriangleMesh(cube_mesh(flip)).sharp_edges(min_angle, 0.0, 1.0);

            // The diagonals of the sides are flat, and not included
            assert_cube_edges(&edges);
        }
    }

    #[test]
    fn cuboid_has_twelve_sharp_edges() {
        let cuboid = Geometry::Cuboid(Cuboid {
            center: SpatialVector([0.5, 0.5, 0.5]),
            half_extents: SpatialVector([0.5, 0.5, 0.5]),
        });

        assert_cube_edges(&cuboid.sharp_edges(0.5, 0.0, 1.0));
    }

    /// The inside of a concave corner is not a sharp convex edge: an L-shaped prism has five
    /// convex edges in its cross section, and one concave
    #[test]
    fn concave_edges_of_a_mesh_are_not_sharp() {
        // L-shaped cross section in the xy-plane, counter-clockwise, extruded along z
        let section = [
            [0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [1.0, 1.0], [1.0, 2.0], [0.0, 2.0]
        ];
        let n = section.len();

        let bottom = |i: usize| SpatialVector([section[i][0], section[i][1], 0.0]);
        let top = |i: usize| SpatialVector([section[i][0], section[i][1], 1.0]);

        let mut triangles = Vec::new();

        // Sides
        for i in 0..n {
            let j = (i + 1) % n;

            triangles.push([bottom(i), bottom(j), top(j)]);
            triangles.push([bottom(i), top(j), top(i)]);
        }

        // Caps, as a fan around the concave corner (vertex 3), which sees all other vertices
        for i in [4, 5, 0, 1] {
            let j = (i + 1) % n;

            triangles.push([top(3), top(i), top(j)]);
            triangles.push([bottom(3), bottom(j), bottom(i)]);
        }

        let edges = Geometry::TriangleMesh(TriangleMesh { triangles }).sharp_edges(0.5, 0.0, 1.0);

        let along_z: Vec<&SharpEdge> = edges.iter()
            .filter(|edge| (edge.end - edge.start)[2].abs() > 0.5)
            .collect();

        // Five convex vertical edges, and six edges around each of the two caps
        assert_eq!(along_z.len(), 5);
        assert_eq!(edges.len(), 5 + 2 * n);

        assert!(along_z.iter().all(|edge| {
            let corner = [edge.start[0], edge.start[1]];

            corner != [1.0, 1.0]
        }));
    }

    #[test]
    fn rounded_disk_has_no_sharp_edges() {
        let disk = |fillet_radius: Float, thickness: Float| Geometry::Disk(Disk {
            center: SpatialVector([0.0; 3]),
            normal: SpatialVector([0.0, 0.0, 1.0]),
            radius: 1.0,
            thickness,
            fillet_radius,
        });

        assert!(disk(0.5, 0.2).sharp_edges(0.5, 0.1, 0.1).is_empty());

        // Two rims, each with segments no longer than the limit
        let edges = disk(0.0, 0.2).sharp_edges(0.5, 0.1, 0.1);

        assert_eq!(edges.len() % 2, 0);
        assert!(edges.iter().all(|edge| (edge.end - edge.start).length() <= 0.1));
        assert!(edges.iter().all(|edge| {
            let radial_distance = (edge.start[0].powi(2) + edge.start[1].powi(2)).sqrt();

            (radial_distance - 1.0).abs() < 1e-5
        }));
    }
}
