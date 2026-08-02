pub mod jacobi_shader;
pub mod restrict_shader;
pub mod prolongate_shader;
pub mod materialize_shader;

use crate::pressure_solver::boundary_conditions::PressureBoundaryConditions;

/// Generates the `BC_X0`/`BC_X1`/`BC_Y0`/`BC_Y1`/`BC_Z0`/`BC_Z1` WGSL consts (0 = ZeroGradient,
/// 1 = ZeroValue) shared by every shader that folds boundary conditions directly into its
/// stencil (jacobi, restrict). Boundary conditions don't depend on grid resolution, so this only
/// needs to be baked in once per solver, not per level.
pub(crate) fn bc_consts_wgsl(boundary_conditions: &PressureBoundaryConditions) -> String {
    let flag = |axis: usize, face: usize| boundary_conditions.condition(axis, face).as_gpu_flag();

    format!(
        "const BC_X0: u32 = {bc_x0}u;\nconst BC_X1: u32 = {bc_x1}u;\nconst BC_Y0: u32 = {bc_y0}u;\nconst BC_Y1: u32 = {bc_y1}u;\nconst BC_Z0: u32 = {bc_z0}u;\nconst BC_Z1: u32 = {bc_z1}u;\n",
        bc_x0 = flag(0, 0),
        bc_x1 = flag(0, 1),
        bc_y0 = flag(1, 0),
        bc_y1 = flag(1, 1),
        bc_z0 = flag(2, 0),
        bc_z1 = flag(2, 1),
    )
}
