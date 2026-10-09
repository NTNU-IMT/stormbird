//! Data immersion close to the sharp convex edges of the geometries with a mirror (slip)
//! correction: the no-slip geometries with the wall model, and, optionally, the slip geometries.
//!
//! The wall model treats the surfaces as slip walls with a wall shear stress, which gives a nearly
//! inviscid flow close to the surfaces. Without a boundary layer, nothing forces the flow to
//! separate at sharp edges, where a real flow always separates. The same is true for the slip
//! geometries, which have no viscous effects at all. To trip the separation, the velocity close to
//! the sharp edges is blended towards zero with the data immersion, in the same way as when the
//! data immersion is used for the whole geometry:
//!
//! 1. The sharp convex edges are taken from the input geometries (see `geometry::sharp_edges`),
//!    and split into short segments. Segments that are covered by another geometry, or that are
//!    on the boundary of the domain, such as the bottom edges of a geometry standing on the
//!    ground, are removed.
//! 2. Each velocity face close to the surface gets a weight, `w`, from its distance to the closest
//!    edge segment: one within `inner_radius_cells` of an edge, smoothly going to zero at
//!    `outer_radius_cells`.
//! 3. The face is blended towards zero with `mu = 1 - w (1 - mu_DI)`, where `mu_DI` is the blend
//!    factor of the data immersion. This is applied after the mirror correction, so that both the
//!    fluid faces close to the edge and the mirrored faces just inside it are damped.
//! 4. The wall shear stress of the wall model is scaled by `1 - w`, as the data immersion already
//!    removes momentum close to the edges. The slip geometries have no wall shear stress.
//!
//! The treatment is on by default, both for the no-slip geometries with the wall model and for the
//! slip geometries, as sharp edges are also troublesome from a numerical point of view: the flow
//! accelerates strongly around them, which gives large velocities in a few cells, and limits the
//! time step. It can be turned off separately for each of them.

use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;
use crate::geometry::Geometry;
use crate::geometry::sharp_edges::SharpEdge;

use super::no_slip_corrections::{NoSlipCorrections, NoSlipEntry};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the data immersion close to the sharp edges of the geometries with a mirror
/// correction. Used both for the no-slip geometries with the wall model, and for the slip
/// geometries, with separate settings.
pub struct SharpEdgeSettings {
    /// Whether the data immersion is applied close to the sharp edges. On by default.
    #[serde(default="SharpEdgeSettings::default_enabled")]
    pub enabled: bool,
    /// The smallest angle, in degrees, between the surface normals on each side of an edge of a
    /// triangle mesh for it to count as sharp. Edges with smaller angles are treated as part of a
    /// smooth, curved surface. The edges of cuboids and disks are always sharp, unless a disk is
    /// rounded with a fillet radius larger than half a cell.
    #[serde(default="SharpEdgeSettings::default_min_angle_degrees")]
    pub min_angle_degrees: Float,
    /// The distance from the edges, as a number of (the largest) cell lengths, within which the
    /// full data immersion is used.
    #[serde(default="SharpEdgeSettings::default_inner_radius_cells")]
    pub inner_radius_cells: Float,
    /// The distance from the edges, as a number of (the largest) cell lengths, beyond which only
    /// the wall model (or the slip condition, for the slip geometries) is used. Between the inner and outer radius, the two are blended smoothly.
    #[serde(default="SharpEdgeSettings::default_outer_radius_cells")]
    pub outer_radius_cells: Float,
}

impl SharpEdgeSettings {
    pub fn default_enabled() -> bool {true}
    pub fn default_min_angle_degrees() -> Float {30.0}
    pub fn default_inner_radius_cells() -> Float {1.0}
    pub fn default_outer_radius_cells() -> Float {3.0}
}

impl Default for SharpEdgeSettings {
    fn default() -> Self {
        Self {
            enabled: Self::default_enabled(),
            min_angle_degrees: Self::default_min_angle_degrees(),
            inner_radius_cells: Self::default_inner_radius_cells(),
            outer_radius_cells: Self::default_outer_radius_cells(),
        }
    }
}

/// The length of the segments the edges are split into, as a number of cell lengths. Short
/// segments make it possible to remove only the parts of an edge that are covered by another
/// geometry, and keep the spatial lookup in `SharpEdgeField` efficient.
const SEGMENT_LENGTH_CELLS: Float = 0.5;

/// How far out from an edge, along its outward direction, the point used to check whether the
/// edge is covered by another geometry or outside the domain is placed, as a number of cell
/// lengths.
const COVERAGE_PROBE_CELLS: Float = 0.5;

#[derive(Debug, Clone)]
/// The sharp edge segments of the no-slip geometries, with a spatial lookup for the distance from
/// any point to the closest segment.
pub struct SharpEdgeField {
    pub edges: Vec<SharpEdge>,
    inner_radius: Float,
    outer_radius: Float,
    bucket_size: Float,
    /// The indices of the edges with their midpoint in each bucket
    buckets: HashMap<[i64; 3], Vec<usize>>,
}

impl SharpEdgeField {
    /// Extracts the sharp edges of `no_slip_geometries`, and removes the parts of them that are
    /// covered by any of `all_geometries`, or that are on the boundary of the domain.
    pub fn build(
        grid: &Grid,
        no_slip_geometries: &[Geometry],
        all_geometries: &[Geometry],
        settings: &SharpEdgeSettings,
    ) -> Self {
        assert!(
            settings.inner_radius_cells >= 0.0 && settings.outer_radius_cells > settings.inner_radius_cells,
            "The sharp edge radii must satisfy 0 <= inner_radius_cells < outer_radius_cells. Got {} and {}",
            settings.inner_radius_cells, settings.outer_radius_cells
        );

        let max_dx = (0..3).map(|axis| grid.cell_length[axis]).fold(0.0, Float::max);

        let segment_length = SEGMENT_LENGTH_CELLS * max_dx;
        let probe_distance = COVERAGE_PROBE_CELLS * max_dx;

        let domain_start = grid.start_point;
        let domain_end = grid.start_point + SpatialVector([
            grid.interior_shape[0] as Float * grid.cell_length[0],
            grid.interior_shape[1] as Float * grid.cell_length[1],
            grid.interior_shape[2] as Float * grid.cell_length[2],
        ]);

        let min_angle = settings.min_angle_degrees.to_radians();

        let segments: Vec<SharpEdge> = no_slip_geometries.iter()
            .flat_map(|geometry| geometry.sharp_edges(min_angle, 0.5 * max_dx, segment_length))
            .flat_map(|edge| edge.subdivided(segment_length))
            .collect();

        let edges: Vec<SharpEdge> = segments.into_par_iter().filter(|segment| {
            let midpoint = 0.5 * (segment.start + segment.end);
            let probe = midpoint + probe_distance * segment.outward;

            let inside_domain = (0..3).all(|axis| {
                probe[axis] > domain_start[axis] && probe[axis] < domain_end[axis]
            });

            inside_domain && Geometry::signed_distance_function_union(all_geometries, probe) >= 0.0
        }).collect();

        let inner_radius = settings.inner_radius_cells * max_dx;
        let outer_radius = settings.outer_radius_cells * max_dx;

        // A segment within `outer_radius` of a point has its midpoint within
        // `outer_radius + segment_length / 2` of it, so it is always in one of the 27 buckets
        // around the bucket of the point
        let bucket_size = outer_radius + segment_length;

        let mut buckets: HashMap<[i64; 3], Vec<usize>> = HashMap::new();

        for (edge_index, edge) in edges.iter().enumerate() {
            let midpoint = 0.5 * (edge.start + edge.end);

            buckets.entry(Self::bucket_key(midpoint, bucket_size)).or_default().push(edge_index);
        }

        Self {
            edges,
            inner_radius,
            outer_radius,
            bucket_size,
            buckets,
        }
    }

    fn bucket_key(point: SpatialVector, bucket_size: Float) -> [i64; 3] {
        std::array::from_fn(|axis| (point[axis] / bucket_size).floor() as i64)
    }

    /// The distance from `point` to the closest edge segment, if it is closer than the outer
    /// radius. Otherwise, the returned distance is larger than the outer radius, but not exact.
    pub fn distance(&self, point: SpatialVector) -> Float {
        let key = Self::bucket_key(point, self.bucket_size);

        let mut min_distance = Float::MAX;

        for di in -1..=1 {
            for dj in -1..=1 {
                for dk in -1..=1 {
                    let neighbor_key = [key[0] + di, key[1] + dj, key[2] + dk];

                    if let Some(edge_indices) = self.buckets.get(&neighbor_key) {
                        for &edge_index in edge_indices {
                            min_distance = min_distance.min(self.edges[edge_index].distance(point));
                        }
                    }
                }
            }
        }

        min_distance
    }

    /// The weight of the data immersion at `point`: one within the inner radius of an edge, and
    /// zero beyond the outer radius, with the same smooth blending as the data immersion between
    /// them.
    pub fn weight(&self, point: SpatialVector) -> Float {
        let distance = self.distance(point);

        let middle = 0.5 * (self.inner_radius + self.outer_radius);
        let half_width = 0.5 * (self.outer_radius - self.inner_radius);

        1.0 - Geometry::blending_function(distance - middle, half_width)
    }
}

#[derive(Debug, Clone, Default)]
/// The data immersion close to the sharp edges, together with the weights it is based on
pub struct SharpEdgeCorrections {
    /// The data immersion of the faces close to the sharp edges. Applied after the mirror
    /// correction.
    pub corrections: NoSlipCorrections,
    /// The weight, `w`, of each face with a non-zero weight, as `(cell_index, w)` for each axis.
    /// Includes the faces where only the wall shear stress is affected. Only used for exporting.
    pub weights: [Vec<(usize, Float)>; 3],
}

impl SharpEdgeCorrections {
    /// Builds the corrections for all faces with a signed distance (from the no-slip geometries)
    /// between `-inner_distance` and `outer_distance`, where `epsilon` is the blending width of
    /// the data immersion. The faces deeper inside the geometries are already set to zero by the
    /// interior corrections of the mirror geometries, and the faces further out are not affected
    /// by either the data immersion or the wall shear stress.
    pub fn build(
        grid: &Grid,
        edge_field: &SharpEdgeField,
        signed_distance_function: &[Float],
        epsilon: Float,
        inner_distance: Float,
        outer_distance: Float,
    ) -> Self {
        let [nxi, nyi, nzi] = grid.interior_shape;

        // Parallel over x-planes, with the entries of each plane in the same order as a sequential
        // loop over the cells, so the result is deterministic
        let face_weights: Vec<(usize, usize, Float, Float)> = (0..nxi).into_par_iter().flat_map_iter(|ii| {
            let mut plane_entries = Vec::new();

            if edge_field.edges.is_empty() {
                return plane_entries;
            }

            for ji in 0..nyi {
                for ki in 0..nzi {
                    let extended_indices = grid.extended_indices_from_interior_indices([ii, ji, ki]);
                    let i_0 = grid.flat_index_on_extended_grid(extended_indices);

                    for axis in 0..3 {
                        let i_p = i_0 + grid.extended_stride[axis];

                        let distance = 0.5 * (signed_distance_function[i_0] + signed_distance_function[i_p]);

                        if distance <= -inner_distance || distance >= outer_distance {
                            continue;
                        }

                        let mut face_center = grid.cell_center_extended(extended_indices);
                        face_center[axis] += 0.5 * grid.cell_length[axis];

                        let weight = edge_field.weight(face_center);

                        if weight > 0.0 {
                            plane_entries.push((i_0, axis, distance, weight));
                        }
                    }
                }
            }

            plane_entries
        }).collect();

        let mut result = Self::default();

        for (cell_index, axis, distance, weight) in face_weights {
            result.weights[axis].push((cell_index, weight));

            let mu_data_immersion = Geometry::blending_function(distance, epsilon);
            let mu = 1.0 - weight * (1.0 - mu_data_immersion);

            if mu < 1.0 {
                result.corrections.entries[axis].push(NoSlipEntry { cell_index, mu });
            }
        }

        result
    }

    /// Combines two sets of corrections, for instance for the no-slip and the slip geometries. For
    /// the faces that are in both, the smallest blend factor and the largest weight are used, so
    /// that each face gets at most one entry.
    pub fn merged(self, other: Self) -> Self {
        let mut weights: [Vec<(usize, Float)>; 3] = Default::default();

        for (axis, (self_weights, other_weights)) in self.weights.into_iter().zip(other.weights).enumerate() {
            let mut weight_per_cell: HashMap<usize, Float> = HashMap::new();

            for (cell_index, weight) in self_weights.into_iter().chain(other_weights) {
                weight_per_cell.entry(cell_index)
                    .and_modify(|current| *current = current.max(weight))
                    .or_insert(weight);
            }

            weights[axis] = weight_per_cell.into_iter().collect();
            weights[axis].sort_by_key(|(cell_index, _)| *cell_index);
        }

        Self {
            corrections: self.corrections.merged(other.corrections),
            weights,
        }
    }

    /// The weights as a staggered vector field on the extended grid, where component `axis` of
    /// cell `i` is the weight of the `axis`-face of the cell. Zero where no weight is stored.
    pub fn weight_field(&self, nr_extended_cells: usize) -> Vec<SpatialVector> {
        let mut field = vec![SpatialVector::default(); nr_extended_cells];

        for (axis, weights) in self.weights.iter().enumerate() {
            for &(cell_index, weight) in weights {
                field[cell_index][axis] = weight;
            }
        }

        field
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::geometry::analytical_shapes::Cuboid;

    fn grid() -> Grid {
        Grid::new(SpatialVector([0.0; 3]), SpatialVector([1.0; 3]), [20, 20, 20])
    }

    fn cuboid(center: [Float; 3], half_extents: [Float; 3]) -> Geometry {
        Geometry::Cuboid(Cuboid {
            center: SpatialVector(center),
            half_extents: SpatialVector(half_extents),
        })
    }

    /// The weight is one at the edges, and zero far from them, also on the flat sides
    #[test]
    fn weight_is_one_at_edges_and_zero_far_away() {
        let grid = grid();
        let box_geometry = [cuboid([0.5, 0.5, 0.5], [0.2, 0.2, 0.2])];

        let field = SharpEdgeField::build(&grid, &box_geometry, &box_geometry, &SharpEdgeSettings::default());

        // At an edge, and at a corner
        assert_eq!(field.weight(SpatialVector([0.7, 0.7, 0.5])), 1.0);
        assert_eq!(field.weight(SpatialVector([0.7, 0.7, 0.7])), 1.0);

        // In the middle of a side, more than three cells from the edges
        assert_eq!(field.weight(SpatialVector([0.71, 0.5, 0.5])), 0.0);

        // Between the inner and outer radius
        let weight = field.weight(SpatialVector([0.7 + 2.0 / 20.0, 0.7, 0.5]));
        assert!(weight > 0.0 && weight < 1.0);
    }

    /// The bottom edges of a box standing on the ground, and the edges where a box sits on top of
    /// another, are not sharp edges of the union of the geometries
    #[test]
    fn covered_edges_and_edges_on_the_domain_boundary_are_removed() {
        let grid = grid();

        let lower = cuboid([0.5, 0.5, 0.2], [0.3, 0.3, 0.2]);
        let upper = cuboid([0.5, 0.5, 0.5], [0.1, 0.1, 0.1]);
        let geometries = [lower, upper];

        let field = SharpEdgeField::build(&grid, &geometries, &geometries, &SharpEdgeSettings::default());

        let covered = |point: [Float; 3]| field.distance(SpatialVector(point)) > 1e-6;

        // Bottom edge of the lower box, on the ground
        assert!(covered([0.5, 0.2, 0.0]));
        // Bottom edge of the upper box, standing on the lower box
        assert!(covered([0.5, 0.4, 0.4]));

        // Top edges of both boxes are kept
        assert!(!covered([0.5, 0.2, 0.4]));
        assert!(!covered([0.5, 0.4, 0.6]));
    }

    /// The corrections are the full data immersion at an edge, and leave faces far from the edges
    /// unchanged
    #[test]
    fn corrections_blend_towards_the_data_immersion_at_edges() {
        let grid = grid();
        let box_geometry = [cuboid([0.5, 0.5, 0.5], [0.2, 0.2, 0.2])];

        let sdf = Geometry::signed_distance_function_on_extended_grid(&box_geometry, &grid);
        let field = SharpEdgeField::build(&grid, &box_geometry, &box_geometry, &SharpEdgeSettings::default());

        let epsilon = 2.0 / 20.0;

        let corrections = SharpEdgeCorrections::build(&grid, &field, &sdf, epsilon, 4.0 / 20.0, epsilon);

        assert!(!corrections.corrections.entries[0].is_empty());

        for (axis, entries) in corrections.corrections.entries.iter().enumerate() {
            for entry in entries {
                let indices = grid.extended_indices_from_flat_index(entry.cell_index);

                let mut face_center = grid.cell_center_extended(indices);
                face_center[axis] += 0.5 / 20.0;

                assert!(field.distance(face_center) < 3.0 / 20.0);
                assert!(entry.mu >= 0.0 && entry.mu < 1.0);
            }
        }
    }
}
