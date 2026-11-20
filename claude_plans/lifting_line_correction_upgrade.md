# Upgrade of the lifting line correction for actuator line simulations

Status legend: `[x]` done, `[~]` in progress, `[ ]` not started.

## Goal

The lifting line correction (Dag and Sørensen, 2020) adds the difference in induced velocity
between two lifting line wake models to the sampled velocity in actuator line simulations: one with
a small (reference) viscous core, and one with a core equal to the force projection width. Before
this work, the only wake was one straight horseshoe vortex per line element.

New features:
1. **Straight multi-panel wake** (`StraightPanels`): still straight, but split into panels, so that
   the reference core can grow downstream towards the projection width. This removes the
   singularities when the tip vortices of an upstream wing pass close to a downstream wing, as the
   two wake models become equal downstream, and the correction from the far wake goes to zero.
2. **Dynamic wake** (`Dynamic`): a free wake where the shape is set by the velocity sampled from the
   CFD flow field at the wake points. Needs a new interface to the CFD solvers (Stormflow and
   OpenFOAM through `cpp_actuator_line`).

Guiding principles from the user: performant but simple implementations that are easy to reason
about; breaking changes are fine (pre 1.0); builders separate from the implementation; keep the
change logs up to date while working.

## Decisions

- Enum-based setup, similar to `SimulationSettings` in the lifting line: settings that are common
  to all wake types stay on `LiftingLineCorrectionBuilder`, and the wake specific settings are in a
  `CorrectionWakeBuilder` enum with the variants `Straight`, `StraightPanels` and `Dynamic`.
- The reference core grows from the small value to the projection width, following a quarter sine
  (same shape as `ViscousCoreLengthEvolution::SinIncrease`). A parameter controls how fast: the
  distance, relative to the average chord length, over which the core grows. The distance is
  measured along the wake, so it means the same for straight and dynamic wakes. The viscous wake
  (core = projection width) has a constant core.
- `StraightPanels` and `Dynamic` both use the existing `DynamicWake` struct for the geometry and
  the panels, with two instances (viscous and reference) with the same geometry.
- The correction is computed with the self-induced / other-induced split from step 0:
  isolated wings use the self-induced part only, `include_wing_interactions` uses both.
- Virtual extensions (end plates) are only supported for the `Straight` wake for now, and the
  setting is moved into that variant. The user wants to think more about how they should work for
  the panel wakes, in particular for the dynamic wake.
- Everything is implemented and tested in Stormbird (Rust) first. The CFD interfaces come last.
  On Windows, only Stormflow is available for testing in a CFD solver.
- The user runs the Python examples and the CFD simulations. Fast `cargo test` runs are fine.

## Step 0: self-induced and other-induced velocities in the lifting line  `[x]`

Committed in `86e485a`. Summary:
- `FrozenWake` owns the split, with the new `InducedVelocities` struct. Blocks per wing, as the line
  elements of each wing are contiguous. `DynamicWake` has no wing logic left.
- `InducedVelocitySettings` in the solvers replaces `velocity_corrections`:
  `neglect_self_induced_velocities` (moved from the dynamic wake), `self_induced_correction`,
  `total_correction`.
- Benchmark (`cargo test --release induced_velocity_benchmark -- --ignored --nocapture`): full time
  steps within 0-3 % of the previous version. Lesson: the panels must be visited in storage order;
  a column-wise traversal was 1.5x slower.

## Step 1: restructure the correction and add `StraightPanels`  `[~]`

Structure: `stormbird/src/actuator_line/corrections/lifting_line/`
- `mod.rs`: `LiftingLineCorrection`, the runtime `CorrectionWake` enum, `update` (geometry and
  state, `&mut self`, called in `do_step` before solving) and `velocity_correction` (`&self`,
  evaluation: initialization ramp and `max_correction_ratio` applied here, as before).
- `builder.rs`: `LiftingLineCorrectionBuilder`, `CorrectionWakeBuilder` and the builders for each
  variant, with `check_settings`.
- `straight.rs`: the horseshoe model (previous behaviour, including virtual extensions). One
  implementation for isolated wings and wing interactions, using the column ranges of each wing.
- `panel_wake.rs`: `PanelWakePair` (viscous and reference `DynamicWake`), the induced velocity
  kernel with the self/other split, and the core length evolution along the wake.
- `tests.rs`

Induced velocity kernel for the panel wakes: for each control point, loop over the streamwise rows
in storage order, and within each row over the column segments before, inside and after the wing of
the point (only inside for isolated wings). The strength of a row is either the given circulation
(first row, or all rows for `StraightPanels`) or the stored strength from earlier time steps
(`Dynamic`). The correction is then a pure function of the inputs and the stored wake state.

Geometry of `StraightPanels`: points at distances `x_k = L (k/N)^p` along the direction of the
sampled velocity at each span point, where `L` is the wake length, `N` the number of panels and `p`
is chosen so that the first panel has the given length. Rebuilt in each call to `update`.

Tasks:
- [ ] Capture the output of the current correction for a multi-wing case, to check that the
      `Straight` variant gives the same results after the restructuring (temporary check)
- [ ] Folder structure, builder and runtime enum
- [ ] `Straight` with one implementation for isolated and all wings
- [ ] `StraightPanels`, with the reference core evolution
- [ ] Tests: `Straight` unchanged; `StraightPanels` with constant core is close to `Straight`;
      the correction from the far wake goes to zero with core evolution; isolated vs all wings for
      a single wing; settings checks
- [ ] Timing of the correction for a typical case
- [ ] Python (`stormbird_setup.actuator_line.corrections`), book, change logs

## Step 2a: dynamic wake in Stormbird  `[ ]`

- `CorrectionWake::Dynamic`: `PanelWakePair` plus the velocity at the wake points and the time of
  the last update.
- The wake is advanced once per new time step in `update` (same check as for the dynamic stall
  models, so that outer iterations in the CFD solver do not advance it): set the strength of the
  first row from the previous circulation, store the sampled velocities, stream the points and the
  strengths, update the core lengths and the panels.
- The wake points move with the sampled CFD velocity only (it already contains the induced
  velocities from the actuator line), so no induced velocity calculation on the wake points.
- New functions on `ActuatorLine`: `wake_points_to_sample`, `set_wake_point_velocity_at_index` and
  a slice version. Non-finite velocities (e.g. points outside the domain) keep the previous value.
- Tests in Stormbird with a prescribed velocity field (no CFD): the dynamic wake converges to the
  straight panel wake for a uniform velocity.

## Step 2b: CFD interfaces  `[ ]`

- `cpp_actuator_line`: `nr_wake_points_to_sample`, `get_wake_point_at_index`,
  `set_wake_point_velocity_at_index`.
- OpenFOAM: sample before `do_step`, `find_cell` every time step (the points move), one batched
  reduce over all wake points (not one per point).
- Stormflow: trilinear stencils for the wake points, fetch the cells through
  `cell_centered_velocity_at_cells` (works for CPU and GPU), interpolate and pass in.
- Optional: export of the correction wake as VTK, for debugging.

## Open questions

- How virtual extensions should work for the panel wakes, in particular the dynamic wake.
