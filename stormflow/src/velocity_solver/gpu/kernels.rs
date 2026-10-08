//! The sources and the shader constants of the velocity solver kernels. The generic kernel helpers
//! are found in `gpu_interface::kernels`.

pub use crate::gpu_interface::kernels::{
    Kernel,
    Binding,
    dispatch_interior,
    dispatch_1d,
    dispatch_plane,
    dispatch_reduce,
    common_constants_wgsl,
};

const COMMON_SRC: &str = include_str!("shaders/common.wgsl");

pub const CONVECT_AND_DIFFUSE_SRC: &str = include_str!("shaders/convect_and_diffuse.wgsl");
/// Only included in the convect and diffuse kernel when the velocity solver uses an eddy viscosity
pub const TURBULENT_STRESS_SRC: &str = include_str!("shaders/turbulent_stress.wgsl");
pub const PRESSURE_RHS_SRC: &str = include_str!("shaders/pressure_rhs.wgsl");
pub const ADD_PRESSURE_GRADIENT_SRC: &str = include_str!("shaders/add_pressure_gradient.wgsl");
pub const NO_SLIP_CORRECTION_SRC: &str = include_str!("shaders/no_slip_correction.wgsl");
pub const SLIP_CORRECTION_SRC: &str = include_str!("shaders/slip_correction.wgsl");
pub const GHOST_CELLS_SRC: &str = include_str!("shaders/ghost_cells.wgsl");
pub const MAX_VELOCITY_SRC: &str = include_str!("shaders/max_velocity.wgsl");
pub const CELL_SAMPLING_SRC: &str = include_str!("shaders/cell_sampling.wgsl");

/// Values that are fixed for the lifetime of a solver, and therefore baked into all shaders as
/// constants
pub struct ShaderConstants {
    pub up_axis: usize,
    pub slip_n: usize,
    pub nr_extended_cells: usize,
}

impl ShaderConstants {
    /// The source that is prepended to all velocity solver kernels: the constants, followed by
    /// the common definitions in common.wgsl.
    pub fn prelude(&self) -> String {
        format!(
            "{}\
             const UP_AXIS: u32 = {}u;\n\
             const SLIP_N: u32 = {}u;\n\
             const N_EXTENDED_CELLS: u32 = {}u;\n\
             {}",
            common_constants_wgsl(),
            self.up_axis,
            self.slip_n,
            self.nr_extended_cells,
            COMMON_SRC
        )
    }
}
