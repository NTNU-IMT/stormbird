use std::collections::HashSet;

use serde::{Serialize, Deserialize};

use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use crate::grid::Grid;
use crate::geometry::{Geometry, WallGeometries};
use crate::gpu_interface::{
    ComputePlatform,
    context::GpuContext
};
use crate::pressure_solver::PressureSolver;

use super::{
    VelocitySolver,
    VelocitySolverSetup,
    boundary_condisitions::VelocityBoundaryConditions,
    slip_mirror_stencils::{
        SlipMirrorStencils, SlipMirrorInterpolationOrder, SLIP_MIRROR_REACH_CELLS,
        MIRROR_INTERIOR_SHIFT_CELLS, mirror_interior_corrections
    },
    no_slip_corrections::NoSlipCorrections,
    wall_model::{NoSlipWallTreatment, WallStressEntries, WALL_STRESS_BAND_CELLS},
    sharp_edges::{SharpEdgeSettings, SharpEdgeField, SharpEdgeCorrections},
    cpu::VelocitySolverCPU,
    gpu::{VelocitySolverGPU, SharedPressureBuffers},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Settings for the velocity solver
pub struct VelocitySolverBuilder {
    /// Where to execute the velocity solver. Independent of where the pressure solver is executed,
    /// but the fewest transfers between the host and the device happen when both are on the same
    /// platform. The turbulence solver, if used, always runs on the same platform as the velocity
    /// solver.
    #[serde(default)]
    pub compute_platform: ComputePlatform,
    /// How the no-slip geometries are treated: with the wall model (default), which is a slip
    /// condition together with the wall shear stress from the log-law (see `wall_model`), or with
    /// the data immersion.
    #[serde(default)]
    pub no_slip_wall_treatment: NoSlipWallTreatment,
    /// Settings for the data immersion close to the sharp convex edges of the no-slip geometries,
    /// which trips the flow separation there. Only used with the wall model, where it is on by
    /// default. See `sharp_edges`.
    #[serde(default)]
    pub sharp_edges: SharpEdgeSettings,
    /// Settings for the data immersion close to the sharp convex edges of the slip geometries,
    /// which trips the flow separation there, while the rest of the slip surfaces stay inviscid.
    /// On by default. See `sharp_edges`.
    #[serde(default)]
    pub slip_sharp_edges: SharpEdgeSettings,
    /// The half width of the band where the data immersion blends the velocity towards zero, as a
    /// number of (the largest) cell lengths. The velocity is blended from zero at this distance
    /// inside the no-slip geometries, to the unchanged velocity at the same distance outside them.
    /// Also used for the wall functions of the turbulence model, which are applied in the fluid
    /// part of the band. Only used with the data immersion, including the data immersion close to
    /// the sharp edges of the geometries with a mirror correction. Values below about one cell make the
    /// blending a step from one face to the next, which can cause oscillations close to the
    /// surfaces.
    #[serde(default="VelocitySolverBuilder::default_no_slip_blending_cells")]
    pub no_slip_blending_cells: Float,
    /// Interpolation order for the mirror correction of the velocity, which is used at the slip
    /// geometries, and at the no-slip geometries with the wall model. Independent of the
    /// equivalent setting for the pressure, though you will usually want to set both to the same
    /// order. Defaults to 2nd order (`Trilinear`), which can never amplify oscillatory errors, and
    /// is more robust for thin walls and sharp edges. `Tricubic` (4th order) is more accurate for
    /// smooth, thick geometries.
    #[serde(default)]
    pub mirror_interpolation_order: SlipMirrorInterpolationOrder,
    /// Optional limiter on the velocity: each velocity component is clipped to at most this factor
    /// times the largest inlet velocity magnitude, both before and after the pressure projection.
    /// Not used by default. This is not physically correct, but a practical safeguard against a
    /// few cells with very large velocities, for instance at sharp convex edges with the wall
    /// model, which would otherwise limit the time step for the whole simulation. The number of
    /// clipped values is available through `Simulation::nr_limited_velocity_values`, and should be
    /// monitored: if it stays large, the limiter is hiding a real problem.
    #[serde(default)]
    pub max_velocity_factor: Option<Float>,
}

impl Default for VelocitySolverBuilder {
    fn default() -> Self {
        Self {
            compute_platform: ComputePlatform::default(),
            no_slip_wall_treatment: NoSlipWallTreatment::default(),
            sharp_edges: SharpEdgeSettings::default(),
            slip_sharp_edges: SharpEdgeSettings::default(),
            no_slip_blending_cells: Self::default_no_slip_blending_cells(),
            mirror_interpolation_order: SlipMirrorInterpolationOrder::default(),
            max_velocity_factor: None,
        }
    }
}

fn max_cell_length(grid: &Grid) -> Float {
    let mut max_dx: Float = 0.0;

    for axis_index in 0..3 {
        max_dx = max_dx.max(grid.cell_length[axis_index]);
    }

    max_dx
}

impl VelocitySolverBuilder {
    pub fn default_no_slip_blending_cells() -> Float {2.0}

    /// The blending width of the data immersion of the no-slip geometries, in meters
    pub fn no_slip_epsilon(&self, grid: &Grid) -> Float {
        assert!(
            self.no_slip_blending_cells > 0.0,
            "no_slip_blending_cells must be positive. Got {}",
            self.no_slip_blending_cells
        );

        self.no_slip_blending_cells * max_cell_length(grid)
    }

    /// The largest allowed magnitude of each velocity component, if the limiter is used: the
    /// `max_velocity_factor` times the largest magnitude of the inlet velocity profile.
    pub fn velocity_limit(&self, boundary_conditions: &VelocityBoundaryConditions) -> Option<Float> {
        self.max_velocity_factor.map(|factor| {
            assert!(factor > 0.0, "max_velocity_factor must be positive. Got {}", factor);

            let max_inlet_velocity = boundary_conditions.inlet_velocity_profile.iter()
                .map(|velocity| velocity.length())
                .fold(0.0, Float::max);

            assert!(
                max_inlet_velocity > 0.0,
                "The velocity limiter needs a non-zero inlet velocity"
            );

            factor * max_inlet_velocity
        })
    }

    /// Precomputes all the geometry corrections, and builds the solver on the selected platform.
    /// On the GPU, the solver is created on the device in `gpu_context`, and shares the buffers of
    /// the pressure solver if it runs on the same device. If `use_eddy_viscosity` is true, the
    /// solver gets an eddy viscosity field for the turbulence solver to write to.
    pub fn build(
        &self,
        grid: &Grid,
        boundary_conditions: VelocityBoundaryConditions,
        initial_velocity: Vec<SpatialVector>,
        no_slip_walls: WallGeometries,
        slip_walls: WallGeometries,
        viscosity: Float,
        density: Float,
        gpu_context: Option<GpuContext>,
        pressure_solver: &PressureSolver,
        use_eddy_viscosity: bool,
    ) -> VelocitySolver {
        let setup = self.build_setup(
            grid, boundary_conditions, no_slip_walls, slip_walls, viscosity, density
        );

        match self.compute_platform {
            ComputePlatform::CPU => VelocitySolver::CPU(
                VelocitySolverCPU::new(setup, initial_velocity, use_eddy_viscosity)
            ),
            ComputePlatform::GPU => {
                let shared_pressure_buffers = match pressure_solver {
                    PressureSolver::MultigridGPU(solver) => Some(SharedPressureBuffers {
                        rhs: solver.rhs_buffer().clone(),
                        pressure: solver.solution_buffer().clone(),
                    }),
                    PressureSolver::MultigridCPU(_) => None,
                };

                VelocitySolver::GPU(
                    VelocitySolverGPU::new(
                        gpu_context.expect("A GPU context is always created for a GPU velocity solver"),
                        grid,
                        setup,
                        &initial_velocity,
                        shared_pressure_buffers,
                        use_eddy_viscosity
                    )
                )
            }
        }
    }

    /// Precomputes everything the velocity solver needs, independent of the platform
    pub fn build_setup(
        &self,
        grid: &Grid,
        boundary_conditions: VelocityBoundaryConditions,
        no_slip_walls: WallGeometries,
        slip_walls: WallGeometries,
        viscosity: Float,
        density: Float,
    ) -> VelocitySolverSetup {
        let max_dx = max_cell_length(grid);

        let velocity_limit = self.velocity_limit(&boundary_conditions);

        if let Some(limit) = velocity_limit {
            println!("Velocity limiter: each velocity component is limited to {:.3} m/s", limit);
        }

        let use_wall_model = self.no_slip_wall_treatment == NoSlipWallTreatment::WallModel;

        // With the wall model, the no-slip geometries get the same mirror correction as the slip
        // geometries, so the stencils are built from the union of both
        let mirror_geometries: Vec<Geometry> = if use_wall_model {
            slip_walls.geometries.iter().chain(no_slip_walls.geometries.iter()).cloned().collect()
        } else {
            slip_walls.geometries.clone()
        };

        let signed_distance_function_mirror: Vec<Float> = if use_wall_model {
            slip_walls.signed_distance_function.iter()
                .zip(&no_slip_walls.signed_distance_function)
                .map(|(slip, no_slip)| slip.min(*no_slip))
                .collect()
        } else {
            slip_walls.signed_distance_function.clone()
        };

        let all_geometries: Vec<Geometry> = slip_walls.geometries.iter()
            .chain(no_slip_walls.geometries.iter())
            .cloned()
            .collect();

        let sharp_edge_field = (use_wall_model && self.sharp_edges.enabled).then(|| {
            println!("Finding sharp edges of the no-slip geometries");
            let edge_field = SharpEdgeField::build(
                grid, &no_slip_walls.geometries, &all_geometries, &self.sharp_edges
            );
            println!("Number of sharp edge segments: {}", edge_field.edges.len());

            edge_field
        });

        let slip_sharp_edge_field = self.slip_sharp_edges.enabled.then(|| {
            println!("Finding sharp edges of the slip geometries");
            let edge_field = SharpEdgeField::build(
                grid, &slip_walls.geometries, &all_geometries, &self.slip_sharp_edges
            );
            println!("Number of sharp edge segments: {}", edge_field.edges.len());

            edge_field
        });

        let wall_stress = if use_wall_model {
            println!("Building wall model");
            WallStressEntries::build(
                grid,
                &no_slip_walls.geometries,
                &no_slip_walls.signed_distance_function,
                &slip_walls.signed_distance_function,
                sharp_edge_field.as_ref(),
            )
        } else {
            WallStressEntries::default()
        };

        let signed_distance_function = no_slip_walls.signed_distance_function;
        let signed_distance_function_slip = slip_walls.signed_distance_function;

        let slip_reach_distance = SLIP_MIRROR_REACH_CELLS * max_dx;

        // The normals are only computed where the slip-mirror stencils use them, and are zero
        // everywhere else
        let normals_slip_surfaces = Geometry::geometry_normals_on_extended_grid(
            &mirror_geometries, grid, 0.1,
            &SlipMirrorStencils::cells_needing_normals(
                grid, &signed_distance_function_mirror, slip_reach_distance
            )
        );

        let slip_epsilon = 4.0 * max_dx;

        println!("Building slip-mirror stencils");
        let mut slip_mirror_stencils = SlipMirrorStencils::build(
            grid,
            &signed_distance_function_mirror,
            &normals_slip_surfaces,
            slip_epsilon,
            slip_reach_distance,
            self.mirror_interpolation_order,
        );

        // The velocity is set to zero deep inside the geometries with a mirror correction, and
        // where the mirror correction is not well defined, see `mirror_interior_corrections`
        println!("Building corrections for the interior of the mirror geometries");
        let (mirror_interior, zeroed_faces) = mirror_interior_corrections(
            grid,
            &mirror_geometries,
            &all_geometries,
            &signed_distance_function_mirror,
            slip_reach_distance,
        );

        let zeroed_faces: HashSet<(usize, usize)> = zeroed_faces.into_iter().collect();

        for (axis, entries) in slip_mirror_stencils.entries.iter_mut().enumerate() {
            entries.retain(|entry| !zeroed_faces.contains(&(entry.cell_index, axis)));
        }

        // The faces deeper inside the mirror geometries than this are already set to zero by the
        // interior corrections
        let interior_zero_distance = (MIRROR_INTERIOR_SHIFT_CELLS + 1.0) * max_dx;

        // For the no-slip geometries, the weights are also needed in the band where the wall shear
        // stress is applied
        let no_slip_sharp_edges = sharp_edge_field.map(|edge_field| {
            println!("Building the data immersion close to the sharp edges of the no-slip geometries");
            SharpEdgeCorrections::build(
                grid,
                &edge_field,
                &signed_distance_function,
                self.no_slip_epsilon(grid),
                interior_zero_distance,
                self.no_slip_epsilon(grid).max(WALL_STRESS_BAND_CELLS * max_dx),
            )
        });

        let slip_sharp_edges = slip_sharp_edge_field.map(|edge_field| {
            println!("Building the data immersion close to the sharp edges of the slip geometries");
            SharpEdgeCorrections::build(
                grid,
                &edge_field,
                &signed_distance_function_slip,
                self.no_slip_epsilon(grid),
                interior_zero_distance,
                self.no_slip_epsilon(grid),
            )
        });

        let sharp_edges = match (no_slip_sharp_edges, slip_sharp_edges) {
            (Some(no_slip), Some(slip)) => Some(no_slip.merged(slip)),
            (no_slip, slip) => no_slip.or(slip),
        };

        let no_slip_corrections = if use_wall_model {
            mirror_interior
        } else {
            println!("Building no-slip corrections");
            NoSlipCorrections::build(
                grid,
                &signed_distance_function,
                self.no_slip_epsilon(grid),
            ).merged(mirror_interior)
        };

        VelocitySolverSetup {
            signed_distance_function,
            signed_distance_function_slip,
            normals_slip_surfaces,
            no_slip_corrections,
            slip_mirror_stencils,
            wall_stress,
            sharp_edges,
            boundary_conditions,
            velocity_limit,
            viscosity,
            density,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_settings_give_the_defaults() {
        let builder: VelocitySolverBuilder = serde_json::from_str("{}").unwrap();

        assert_eq!(builder.compute_platform, ComputePlatform::CPU);
        assert_eq!(builder.no_slip_wall_treatment, NoSlipWallTreatment::WallModel);
        assert_eq!(builder.no_slip_blending_cells, 2.0);
        assert_eq!(builder.mirror_interpolation_order, SlipMirrorInterpolationOrder::Trilinear);
        assert_eq!(builder.max_velocity_factor, None);
        assert_eq!(builder.sharp_edges, SharpEdgeSettings::default());
        assert!(builder.sharp_edges.enabled);
        assert_eq!(builder.slip_sharp_edges, SharpEdgeSettings::default());

        let builder: VelocitySolverBuilder = serde_json::from_str(
            r#"{"slip_sharp_edges": {"enabled": false}}"#
        ).unwrap();

        assert!(!builder.slip_sharp_edges.enabled);
        assert_eq!(builder.slip_sharp_edges.outer_radius_cells, 3.0);
    }

    #[test]
    fn no_slip_epsilon_uses_the_largest_cell_length() {
        let grid = Grid::new(SpatialVector([0.0; 3]), SpatialVector([8.0, 4.0, 2.0]), [8, 8, 8]);

        let builder = VelocitySolverBuilder {
            no_slip_blending_cells: 1.5,
            ..Default::default()
        };

        assert_eq!(builder.no_slip_epsilon(&grid), 1.5);
    }
}
