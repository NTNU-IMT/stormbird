# Known missing features and potential fixes

A list of known issues and missing features that are deferred to later versions. Each item has a
short description of the problem and, where known, an idea for the fix. Remove an item when it is
fixed, and mention the fix in the change log.

## Lifting line

- **Convergence test of the `AngleOfAttack` update in the iterative solver.** With
  `variable_to_update: AngleOfAttack`, the solver never stops before
  `max_iterations_per_time_step` (`solvers/iterative.rs`, `update_angle_of_attack`): both
  convergence checks set `converged = false`. Changing them to `true` is not enough, as the residual
  in this mode is always close to zero (the circulation is computed from the same damped angles that
  the residual compares it with). Needs a real stop test, e.g., the largest difference between the
  new and the damped angle of attack. Documented in the book (`lifting_line/solver.md`).

## Actuator line

- **Body-fixed output coordinate system.** The actuator line simulations are currently always run
  with the output in the global coordinate system. With `output_coordinate_system: Body` and a
  rotated model, the projected forces would be in the body-fixed frame, while the CFD grid is
  global: `ActuatorLine::update_sectional_forces_to_project` computes the circulatory forces from
  `simulation_result.force_input.velocity`, and takes the drag from
  `simulation_result.sectional_forces`, which both follow the output coordinate system. With the
  default `realign_sectional_forces = true`, only the magnitude is used, but with
  `realign_sectional_forces = false` or `project_sectional_drag = true`, body-fixed vectors are
  projected directly. Fix idea: always compute the projected forces from the global velocity and
  the global span lines, independent of the output coordinate system.

## Controller

- **Sign of the apparent wind direction.** The controller measures the apparent wind direction
  around the span direction of each wing, while the wind environment (and the inflow corrections)
  measure it around the `wind_rotation_axis`. With the default wind environment and sails built
  from the bottom up, the two have opposite signs: a wind from starboard is negative in the
  controller. Set points that are mirrored between the two sides are not affected. Documented in
  the book (`utils/control_system.md`, "Sign of the apparent wind direction") and in the doc
  comment of `ControllerSetPoints::apparent_wind_directions_data`. A fix changes the meaning of
  set points that are not mirrored, including the sign of the default rotor sail set points in
  stormbird_setup (`default_sails/rotor_sails.py`), which are currently given with the controller
  convention: spin ratio -3 for negative directions, i.e., wind from starboard.
- **Moving average together with the effective angle of attack.** With
  `use_effective_angle_of_attack = true`, the controller adds the full error in the angle of attack
  to the local wing angle in each update, i.e., an integrator with a gain of one. With a
  `moving_average_window_size` of 5 or more, and `time_steps_between_updates = 1`, the averaged
  input lags so much that the loop becomes unstable, even when the angle of attack responds
  directly to the wing angle (spectral radius of the closed loop above one for windows of 5 or
  more). It is stable when `time_steps_between_updates` is at least the window size, as the
  window then only contains inputs after the last change. Fix ideas: a gain below one for the
  effective angle of attack, or a warning or error for the unstable combinations.
