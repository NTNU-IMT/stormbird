# Upgrade of the lifting line correction for actuator line simulations

Status legend: `[x]` done, `[~]` in progress, `[ ]` not started.

**Current state:** steps 0 and 1 are done and committed (`86e485a`, `a900e6a`). Step 2a is next,
see its section for the detailed design. Run the tests with `cargo test --release` in `stormbird`
(also with `--features parallel` and `--features single_precision`).

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

## Step 1: restructure the correction and add `StraightPanels`  `[x]`

Structure: `stormbird/src/actuator_line/corrections/lifting_line/`
- `mod.rs`: `LiftingLineCorrection` (shared `CorrectionSettings`, `initialization_time`,
  `max_correction_ratio`, `wake: CorrectionWake`), `update` (geometry and state, `&mut self`,
  called from `ActuatorLine::solve` through `update_lifting_line_correction`) and
  `velocity_correction` (`&self`, evaluation; the initialization ramp and `max_correction_ratio`
  are applied here, as before).
- `builder.rs`: `LiftingLineCorrectionBuilder`, `CorrectionWakeBuilder` (`Straight`,
  `StraightPanels`), `StraightWakeBuilder`, `StraightPanelWakeBuilder`, and `check_settings`.
- `straight.rs`: horseshoe model, previous behaviour including virtual extensions. One
  implementation for isolated wings and wing interactions: the columns (line elements) included for
  a control point are the wing's own range, or all.
- `straight_panels.rs`: `StraightPanelWake`, geometry `x_k = L (k/N)^p` along the sampled velocity
  direction, rebuilt in each `update`.
- `panel_wake.rs`: `PanelWakePair` (viscous and reference `DynamicWake`, same geometry), the
  induced velocity kernel, and `core_lengths_along_wake` (distance along the streamwise lines,
  average of the four panel corners, sine blend). Shared with the dynamic wake in step 2a. The
  kernel takes `nr_rows_with_given_strength`: all rows for `StraightPanels`, the first row for
  `Dynamic` (the rest use the stored strengths).
- `tests.rs`

Results:
- `Straight` gives the same correction as before the restructuring (max relative difference 6e-16
  for a three-wing case, isolated and with interactions; checked with a temporary test against the
  output of the old code).
- `StraightPanels` with constant core differs from `Straight` by up to 3.6 % at the outermost
  control points. Cause: the viscous core term in `VortexLine` uses the distance to the closest
  point on the segment, so a trailing vortex split into segments is damped less than one long line
  close to its start. Not a bug, documented in the test and the book.
- With `reference_core_evolution_length_factor = 2` and a downstream wing 10 chords behind, the
  correction from the upstream wing at the downstream wing goes from 1.0 to 2e-15.
- Timing (3 wings x 40 elements, symmetry, per call of update + correction):
  `Straight` 0.6 ms isolated / 1.8 ms with interactions; `StraightPanels` (50 panels) 5.4 / 14.8 ms
  serial, 2.0 / 4.4 ms with `parallel`. Run with
  `cargo test --release lifting_line_correction_benchmark -- --ignored --nocapture`.
- Python (`stormbird_setup.actuator_line`: `StraightWakeBuilder`, `StraightPanelWakeBuilder`),
  book (`actuator_line/corrections.md`) and change logs are updated. Python only syntax-checked
  (no pydantic in the local Python).

Possible later optimization, if the cost of `StraightPanels` matters: fewer panels (the cost is
proportional), or skipping the panels beyond the evolution length when the reference and the
projected circulation are equal (the two wakes then cancel exactly there).

## Step 2a: dynamic wake in Stormbird  `[ ]`  <- NEXT STEP

Paused by the user before any code was written (the working tree was clean after `a900e6a`). The
design below is ready to implement. Confirm with the user before starting, as they may want to
adjust it.

### Builder (`builder.rs`)
New variant `CorrectionWakeBuilder::Dynamic(DynamicCorrectionWakeBuilder)` with:
- `nr_panels_per_line_element` (default 100, at least 2)
- `first_panel_relative_length` (default 0.75, positive), relative to the average chord length
- `last_panel_relative_length` (default 1.0, positive)
- `first_wake_points_direction: FirstWakePointsDirection` (default `Freestream`)
- `shape_damping_factor` (default 0.0, in [0, 1])
- `reference_core_evolution_length_factor: Option<Float>` (same meaning as for `StraightPanels`)
- `far_field_ratio` (default 5.0)

Built into a `DynamicWakeBuilder` with `steady_state_strength_update: false` and
`ratio_of_wake_affected_by_induced_velocities: 0.0`, then `PanelWakePair::new` (which sets the
core lengths and the symmetry condition). Add the checks to `check_settings`.

### Runtime (`dynamic.rs`, new file)
```rust
pub struct DynamicCorrectionWake {
    pub wakes: PanelWakePair,
    /// Sampled velocity at the points of the wake (same geometry for both wakes)
    pub wake_points_velocity: Vec<SpatialVector>,
    /// Time of the last update, to detect new time steps
    last_update_time: Option<Float>,
}
```
Add `CorrectionWake::Dynamic(DynamicCorrectionWake)`.

`update` needs more input than today: the previous circulation (reference and projected), the time
and the time step. Change `LiftingLineCorrection::update` to take them, and
`ActuatorLine::update_lifting_line_correction(time, time_step)` to pass them, using the existing
private `last_circulation_strengths`. `ActuatorLine::solve` already has `time` and `time_step`
(rename `_time_step`).

Logic in `DynamicCorrectionWake::update`:
1. First call (`last_update_time` is `None`): mean of the sampled control point velocities `U`;
   for both wakes `initialize_with_velocity_and_time_step(lfm, U, time_step)` (straight wake with
   the full length, zero strength); `wake_points_velocity = vec![U; nr_points]`;
   `wakes.update_panels(settings)`.
2. New time step (`time > last_update_time`), for both wakes, with the projected circulation for
   the viscous wake and the reference circulation for the reference wake:
   - `wake.update_after_solving(previous_circulation, &wake_points_velocity)`: first row strength
     and the stored velocity at the points (no induced velocities, as the ratio is zero)
   - `wake.update_wake_points_before_solving(time_step, lfm, ctrl_points_velocity)`: sync to the
     wing geometry, Euler step of the points, first and last row
   - `wake.stream_strength_values_downstream()`

   then `wakes.update_panels(settings)` (core lengths along the wake, and the panel data).
3. Same time as the last update (outer iterations in the CFD solver): do nothing.

Evaluation: `wakes.induced_velocity_difference(..., nr_rows_with_given_strength = 1)`, so the first
row gets the current circulation and the rest use the stored history.

The wake points move with the sampled CFD velocity only (it already contains the induced
velocities from the actuator line). Euler forward: the velocity sampled at time t, at the positions
from the last update, moves the points to the positions used at time t.

### `ActuatorLine` interface (Rust, used by the CFD interfaces in step 2b)
- `wake_points_to_sample(&self) -> &[SpatialVector]`: the points of the reference wake, empty
  unless the wake is `Dynamic`
- `nr_wake_points_to_sample(&self) -> usize`
- `set_wake_point_velocity_at_index(&mut self, index, velocity)` and
  `set_wake_points_velocity(&mut self, &[SpatialVector])`. Non-finite values (e.g. points outside
  the domain, OpenFOAM returns VGREAT) keep the previous value.

### Tests (no CFD, prescribed velocities)
- For a uniform velocity and constant circulation, the dynamic correction converges (after more
  steps than panels) to the `Straight` correction with the same wake length, within a few percent
  (see the segmentation effect in step 1).
- Calling `update` several times with the same time does not move the wake; a new time does.
- The wake follows the sampled wake point velocities, e.g. a sideways component moves the points
  sideways.
- The strength of row k is the circulation from k time steps earlier.
- Non-finite velocities are ignored by the setter.
- Settings checks and JSON.

Then: book (`actuator_line/corrections.md`, add the dynamic wake and the sampling of the wake
points), change logs (Stormbird, stormbird_setup), Python (`DynamicCorrectionWakeBuilder` in
`stormbird_setup.actuator_line.corrections`, and the `wake` union and (de)serializer in
`LiftingLineCorrectionBuilder`), this plan.

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

## Notes

- `controller::set_points::tests::limit_angle_takes_the_shortest_way_across_pi` fails with the
  `single_precision` feature (used by Stormflow). It is from 0.13.1 and not related to this work.
  All other tests pass in single and double precision, with and without `parallel`.
