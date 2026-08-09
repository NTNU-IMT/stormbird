use stormath::type_aliases::Float;

use super::INTERIOR_OFFSET;
use super::finite_difference;

/// How far beyond the extended grid the coordinate tables reach. The widest stencil below touches
/// nodes 3 cells away from the cell it belongs to, so the tables have to be valid for every
/// extended index offset by +/- 3.
const STENCIL_REACH: usize = 3;

/// Precomputed finite-difference and interpolation weights along a single axis.
///
/// Because the grid is a Cartesian tensor product, every stencil coefficient depends only on the
/// index along one axis, so these tables are one-dimensional. They are indexed by the **extended**
/// index along that axis, and are computed for the entire extended range (ghost cells included)
/// from the linearly extrapolated ghost geometry.
///
/// Two families of sample locations occur in the staggered (MAC) layout, and each stencil is named
/// after the one it reads from and the one it writes to:
///
/// - **center**: cell centers, where the pressure and the cell-centered body force live. Offset `k`
///   is the center of the cell `k` cells away.
/// - **face**: the positive faces, where `velocity[i][axis]` lives. Offset `k` is the positive face
///   of the cell `k` cells away.
///
/// On a uniform grid every table below collapses to the classical constant coefficients the solver
/// used previously (see `finite_difference`'s tests), so this is a strict generalization.
#[derive(Debug, Clone)]
pub struct AxisStencils {
    /// Interpolation of cell-centered samples (offsets -1..=2) onto the positive face.
    pub interpolate_center_to_face: Vec<[Float; 4]>,
    /// Interpolation of face samples (offsets -2..=1) onto the cell center.
    pub interpolate_face_to_center: Vec<[Float; 4]>,
    /// First derivative at the positive face from cell-centered samples (offsets -1..=2). This is
    /// the pressure gradient of the projection step.
    pub gradient_center_to_face: Vec<[Float; 4]>,
    /// First derivative at the cell center from face samples (offsets -2..=1). This is the
    /// divergence of the projection step, and the exact adjoint counterpart of
    /// `gradient_center_to_face`.
    pub divergence_face_to_center: Vec<[Float; 4]>,
    /// 4th order accurate second derivative at the cell center from cell-centered samples
    /// (offsets -2..=2). This is the Poisson operator the pressure solve inverts, and the viscous
    /// term for the velocity components that are cell-centered along this axis.
    pub second_derivative_center: Vec<[Float; 5]>,
    /// 4th order accurate second derivative at the positive face from face samples
    /// (offsets -2..=2). The viscous term for the velocity component staggered along this axis.
    pub second_derivative_face: Vec<[Float; 5]>,
    /// 2nd order accurate second derivative at the cell center from cell-centered samples
    /// (offsets -1..=1), for the 2nd order residual check.
    pub second_derivative_center_low_order: Vec<[Float; 3]>,
    /// Upwind-biased first derivative at the cell center, cell-centered samples at offsets
    /// -3..=1, for a transport velocity pointing in the `+` direction.
    pub upwind_plus_center: Vec<[Float; 5]>,
    /// Mirror of `upwind_plus_center`: offsets -1..=3, transport velocity pointing in `-`.
    pub upwind_minus_center: Vec<[Float; 5]>,
    /// Upwind-biased first derivative at the positive face, face samples at offsets -3..=1.
    pub upwind_plus_face: Vec<[Float; 5]>,
    /// Mirror of `upwind_plus_face`: offsets -1..=3.
    pub upwind_minus_face: Vec<[Float; 5]>,
}

impl AxisStencils {
    /// Builds every table for one axis from that axis' interior points (the cell vertices).
    ///
    /// `vertex_position` supplies the position of a possibly out-of-range vertex, extrapolated with
    /// the nearest interior cell's length, which is what defines the ghost geometry.
    pub fn new(extended_length: usize, vertex_position: impl Fn(isize) -> Float) -> Self {
        // Padded coordinate tables so the stencil gather below never needs a bounds check or an
        // index conversion: entry `p` corresponds to signed interior cell index
        // `p - INTERIOR_OFFSET - STENCIL_REACH`.
        let padded_length = extended_length + 2 * STENCIL_REACH;
        let index_shift = (INTERIOR_OFFSET + STENCIL_REACH) as isize;

        let mut centers: Vec<Float> = Vec::with_capacity(padded_length);
        let mut faces: Vec<Float> = Vec::with_capacity(padded_length);

        for p in 0..padded_length {
            let cell_index = p as isize - index_shift;

            let negative_vertex = vertex_position(cell_index);
            let positive_vertex = vertex_position(cell_index + 1);

            centers.push(0.5 * (negative_vertex + positive_vertex));
            faces.push(positive_vertex);
        }

        let gather = |coordinates: &[Float], base: usize, first_offset: isize, count: usize| -> Vec<Float> {
            (0..count)
                .map(|n| coordinates[(base as isize + first_offset + n as isize) as usize])
                .collect()
        };

        let mut out = Self {
            interpolate_center_to_face: Vec::with_capacity(extended_length),
            interpolate_face_to_center: Vec::with_capacity(extended_length),
            gradient_center_to_face: Vec::with_capacity(extended_length),
            divergence_face_to_center: Vec::with_capacity(extended_length),
            second_derivative_center: Vec::with_capacity(extended_length),
            second_derivative_face: Vec::with_capacity(extended_length),
            second_derivative_center_low_order: Vec::with_capacity(extended_length),
            upwind_plus_center: Vec::with_capacity(extended_length),
            upwind_minus_center: Vec::with_capacity(extended_length),
            upwind_plus_face: Vec::with_capacity(extended_length),
            upwind_minus_face: Vec::with_capacity(extended_length),
        };

        for extended_index in 0..extended_length {
            let base = extended_index + STENCIL_REACH;

            let center_target = centers[base];
            let face_target = faces[base];

            let nodes4 = |coordinates: &[Float], first_offset: isize| -> [Float; 4] {
                let gathered = gather(coordinates, base, first_offset, 4);

                [gathered[0], gathered[1], gathered[2], gathered[3]]
            };

            let nodes5 = |coordinates: &[Float], first_offset: isize| -> [Float; 5] {
                let gathered = gather(coordinates, base, first_offset, 5);

                [gathered[0], gathered[1], gathered[2], gathered[3], gathered[4]]
            };

            let nodes3 = |coordinates: &[Float], first_offset: isize| -> [Float; 3] {
                let gathered = gather(coordinates, base, first_offset, 3);

                [gathered[0], gathered[1], gathered[2]]
            };

            out.interpolate_center_to_face.push(
                finite_difference::weights(0, face_target, nodes4(&centers, -1))
            );
            out.interpolate_face_to_center.push(
                finite_difference::weights(0, center_target, nodes4(&faces, -2))
            );
            out.gradient_center_to_face.push(
                finite_difference::weights(1, face_target, nodes4(&centers, -1))
            );
            out.divergence_face_to_center.push(
                finite_difference::weights(1, center_target, nodes4(&faces, -2))
            );
            out.second_derivative_center.push(
                finite_difference::weights(2, center_target, nodes5(&centers, -2))
            );
            out.second_derivative_face.push(
                finite_difference::weights(2, face_target, nodes5(&faces, -2))
            );
            out.second_derivative_center_low_order.push(
                finite_difference::weights(2, center_target, nodes3(&centers, -1))
            );
            out.upwind_plus_center.push(
                finite_difference::weights(1, center_target, nodes5(&centers, -3))
            );
            out.upwind_minus_center.push(
                finite_difference::weights(1, center_target, nodes5(&centers, -1))
            );
            out.upwind_plus_face.push(
                finite_difference::weights(1, face_target, nodes5(&faces, -3))
            );
            out.upwind_minus_face.push(
                finite_difference::weights(1, face_target, nodes5(&faces, -1))
            );
        }

        out
    }
}
