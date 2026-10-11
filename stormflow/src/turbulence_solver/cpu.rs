use stormath::spatial_vector::SpatialVector;
use stormath::type_aliases::Float;

use rayon::prelude::*;

use crate::grid::Grid;

use super::TurbulenceSolverSetup;
use super::models::TurbulenceModel;
use super::models::realizable_k_epsilon::{self, RealizableKEpsilon};
use super::boundary_conditions::TurbulenceBoundaryConditions;
use super::transport::ParallelWriteSlice;

/// Turbulence solver executed on the CPU, with all fields stored on the host. All transported and
/// auxiliary fields are stored field-major on the extended grid: value `f` of cell `i` is at index
/// `f * nr_extended_cells + i`.
pub struct TurbulenceSolverCPU {
    pub setup: TurbulenceSolverSetup,
    /// The transported fields
    pub fields: Vec<Float>,
    /// The transported fields at the start of the time step
    fields_old: Vec<Float>,
    /// The result of the current Jacobi iteration
    fields_next: Vec<Float>,
    pub auxiliary: Vec<Float>,
    /// The fixed values of the wall function field, for each wall function entry
    wall_values: Vec<Float>,
    /// Temporary storage for the mirror corrections, for each mirror entry and field
    mirror_values: Vec<Float>,
    y_plus_lam: Float,
    nr_extended_cells: usize,
}

/// Executes `kernel` for the flat extended index of each interior cell, in parallel over x-planes
fn par_for_each_interior_cell(grid: &Grid, kernel: impl Fn(usize) + Sync) {
    let [nxi, nyi, nzi] = grid.interior_shape;

    (0..nxi).into_par_iter().for_each(|ii| {
        for ji in 0..nyi {
            let mut i_0 = grid.flat_index_on_extended_grid_from_interior_indices([ii, ji, 0]);

            for _ in 0..nzi {
                kernel(i_0);

                i_0 += 1;
            }
        }
    });
}

impl TurbulenceSolverCPU {
    pub fn new(setup: TurbulenceSolverSetup, grid: &Grid) -> Self {
        let nr_extended_cells = grid.nr_extended_cells();

        let fields = setup.boundary_conditions.initial_fields(grid);

        let auxiliary = vec![0.0; setup.model.nr_auxiliary_fields() * nr_extended_cells];
        let wall_values = vec![0.0; setup.wall_treatment.wall_functions.len()];
        let mirror_values = vec![0.0; setup.wall_treatment.mirror.len() * setup.model.nr_fields()];

        let y_plus_lam = setup.wall_function_constants.y_plus_lam();

        Self {
            setup,
            fields_old: fields.clone(),
            fields_next: fields.clone(),
            fields,
            auxiliary,
            wall_values,
            mirror_values,
            y_plus_lam,
            nr_extended_cells,
        }
    }

    pub fn initialize(&mut self, grid: &Grid, velocity: &[SpatialVector], eddy_viscosity: &mut [Float]) {
        self.apply_mirror_corrections(grid, true);

        self.setup.boundary_conditions.set_ghost_cells(grid, velocity, &mut self.fields);

        self.update_eddy_viscosity(grid, velocity, eddy_viscosity);
    }

    pub fn update(
        &mut self,
        grid: &Grid,
        velocity: &[SpatialVector],
        eddy_viscosity: &mut [Float],
        time_step: Float
    ) {
        self.fields_old.copy_from_slice(&self.fields);

        self.compute_auxiliary_fields(grid, velocity, eddy_viscosity);
        self.compute_wall_functions(grid, velocity);

        Self::apply_wall_function_values(&self.setup, &self.wall_values, &mut self.fields, self.nr_extended_cells);

        for _ in 0..self.setup.nr_jacobi_iterations {
            self.transport_iteration(grid, velocity, eddy_viscosity, time_step);

            Self::apply_wall_function_values(&self.setup, &self.wall_values, &mut self.fields_next, self.nr_extended_cells);
            self.apply_mirror_corrections(grid, false);
            self.setup.boundary_conditions.set_ghost_cells(grid, velocity, &mut self.fields_next);

            std::mem::swap(&mut self.fields, &mut self.fields_next);
        }

        self.update_eddy_viscosity(grid, velocity, eddy_viscosity);
    }

    /// The transported field with index `field_index`
    pub fn field(&self, field_index: usize) -> &[Float] {
        let n = self.nr_extended_cells;

        &self.fields[field_index * n..(field_index + 1) * n]
    }

    // ---------------------- Model specific steps ----------------------

    fn compute_auxiliary_fields(&mut self, grid: &Grid, velocity: &[SpatialVector], eddy_viscosity: &[Float]) {
        let n = self.nr_extended_cells;
        let fields_old = &self.fields_old;
        let auxiliary = ParallelWriteSlice::new(&mut self.auxiliary);

        match &self.setup.model {
            TurbulenceModel::RealizableKEpsilon(model) => {
                par_for_each_interior_cell(grid, |i_0| {
                    let values = model.auxiliary_kernel(i_0, grid, velocity, eddy_viscosity, fields_old, n);

                    for (aux_index, value) in values.into_iter().enumerate() {
                        // Safe: each cell only writes its own index
                        unsafe { auxiliary.write(aux_index * n + i_0, value); }
                    }
                });
            }
        }
    }

    /// Computes the wall function values, which overwrite the production in the auxiliary fields,
    /// and are stored in `wall_values` for the wall function field.
    fn compute_wall_functions(&mut self, grid: &Grid, velocity: &[SpatialVector]) {
        let n = self.nr_extended_cells;
        let entries = &self.setup.wall_treatment.wall_functions;

        let results: Vec<[Float; 2]> = match &self.setup.model {
            TurbulenceModel::RealizableKEpsilon(_) => entries.par_iter().map(|entry| {
                RealizableKEpsilon::wall_function_kernel(
                    entry,
                    grid,
                    velocity,
                    &self.fields_old,
                    n,
                    self.setup.viscosity,
                    &self.setup.wall_function_constants,
                    self.y_plus_lam
                )
            }).collect(),
        };

        for ((entry, [production, wall_value]), stored_value) in entries.iter()
            .zip(results)
            .zip(self.wall_values.iter_mut())
        {
            self.auxiliary[realizable_k_epsilon::PRODUCTION * n + entry.cell_index] = production;
            *stored_value = wall_value;
        }
    }

    /// One Jacobi iteration of the transport equations, from `fields` into `fields_next`
    fn transport_iteration(
        &mut self,
        grid: &Grid,
        velocity: &[SpatialVector],
        eddy_viscosity: &[Float],
        time_step: Float
    ) {
        let n = self.nr_extended_cells;
        let fields = &self.fields;
        let fields_old = &self.fields_old;
        let auxiliary = &self.auxiliary;
        let viscosity = self.setup.viscosity;
        let scheme = self.setup.convection_scheme;

        let fields_next = ParallelWriteSlice::new(&mut self.fields_next);

        match &self.setup.model {
            TurbulenceModel::RealizableKEpsilon(model) => {
                par_for_each_interior_cell(grid, |i_0| {
                    let values = model.transport_kernel(
                        i_0, grid, velocity, eddy_viscosity, fields, fields_old, auxiliary, n,
                        viscosity, time_step, scheme
                    );

                    for (field_index, value) in values.into_iter().enumerate() {
                        // Safe: each cell only writes its own index
                        unsafe { fields_next.write(field_index * n + i_0, value); }
                    }
                });
            }
        }
    }

    /// Computes the eddy viscosity from the transported fields, damps it inside the no-slip
    /// geometries, and sets the ghost cells.
    fn update_eddy_viscosity(&self, grid: &Grid, velocity: &[SpatialVector], eddy_viscosity: &mut [Float]) {
        let n = self.nr_extended_cells;
        let fields = &self.fields;
        let max_eddy_viscosity = self.setup.max_eddy_viscosity;

        {
            let output = ParallelWriteSlice::new(eddy_viscosity);

            match &self.setup.model {
                TurbulenceModel::RealizableKEpsilon(model) => {
                    par_for_each_interior_cell(grid, |i_0| {
                        let value = model.eddy_viscosity_kernel(i_0, grid, velocity, fields, n, max_eddy_viscosity);

                        // Safe: each cell only writes its own index
                        unsafe { output.write(i_0, value); }
                    });
                }
            }
        }

        for entry in &self.setup.wall_treatment.damping {
            eddy_viscosity[entry.cell_index] *= entry.mu;
        }

        TurbulenceBoundaryConditions::set_zero_gradient_ghost_cells(grid, eddy_viscosity);
    }

    // ---------------------- Model independent steps ----------------------

    /// Fixes the wall function field to the precomputed wall function values
    fn apply_wall_function_values(
        setup: &TurbulenceSolverSetup,
        wall_values: &[Float],
        fields: &mut [Float],
        nr_extended_cells: usize
    ) {
        let offset = setup.model.wall_function_field() * nr_extended_cells;

        for (entry, &value) in setup.wall_treatment.wall_functions.iter().zip(wall_values) {
            fields[offset + entry.cell_index] = value;
        }
    }

    /// Sets the transported fields in the cells inside the geometries to the values at their
    /// mirrored image points, either in `fields` or in `fields_next`. All the values are sampled
    /// before any is written, so that the result is independent of the order of the entries.
    fn apply_mirror_corrections(&mut self, grid: &Grid, current_fields: bool) {
        let entries = &self.setup.wall_treatment.mirror;

        if entries.is_empty() {
            return;
        }

        let n = self.nr_extended_cells;
        let nr_fields = self.setup.model.nr_fields();
        let stride = grid.extended_stride;

        let fields = if current_fields { &mut self.fields } else { &mut self.fields_next };

        self.mirror_values.par_chunks_mut(nr_fields)
            .zip(entries.par_iter())
            .for_each(|(values, entry)| {
                for (field_index, value) in values.iter_mut().enumerate() {
                    *value = entry.stencil.sample_scalar(&fields[field_index * n..(field_index + 1) * n], stride);
                }
            });

        for (values, entry) in self.mirror_values.chunks_exact(nr_fields).zip(entries) {
            for (field_index, &value) in values.iter().enumerate() {
                fields[field_index * n + entry.cell_index] = value;
            }
        }
    }
}
