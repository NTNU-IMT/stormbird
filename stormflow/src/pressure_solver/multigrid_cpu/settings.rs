

#[derive(Debug, Clone)]
pub struct MultigridSettings {
    pub nr_v_cycles: usize,
    pub nr_smooth_iterations: usize,
    pub compute_residual_after_solve: bool
}