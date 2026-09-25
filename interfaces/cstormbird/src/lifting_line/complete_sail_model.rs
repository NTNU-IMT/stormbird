
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::{
    catch_panic,
    set_last_error,
};
use crate::wind::{
    WindCondition,
    wind_condition_ref,
};
use crate::results::{
    SimulationResult,
    SingleSailResult,
    box_simulation_result,
};

use stormbird::{
    lifting_line::complete_sail_model::CompleteSailModel as CompleteSailModelImpl,
    common_utils::results::simulation::SimulationResult as SimulationResultImpl,
};

use stormath::spatial_vector::SpatialVector;

/// Opaque pointer structure to the CompleteSailModel
#[repr(C)]
pub struct CompleteSailModel {
    _private: [u8; 0],
}

/// Creates a new CompleteSailModel from a JSON setup string.
///
/// Returns NULL on error, in which case `stormbird_last_error_message` describes what went wrong.
///
/// # Safety
/// - `setup_string` must be a valid null-terminated C string
/// - The returned pointer must be freed with `complete_sail_model_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_new(setup_string: *const c_char) -> *mut CompleteSailModel {
    if setup_string.is_null() {
        set_last_error("complete_sail_model_new: the setup string pointer is null");

        return std::ptr::null_mut();
    }

    let setup_str = match unsafe { CStr::from_ptr(setup_string) }.to_str() {
        Ok(s) => s.to_string(),
        Err(error) => {
            set_last_error(
                format!("complete_sail_model_new: the setup string is not valid UTF-8: {}", error)
            );

            return std::ptr::null_mut();
        }
    };

    // Building a model from a valid setup string can still panic, for instance if the number of
    // local wing angles does not match the number of wings. A panic that unwinds out of this
    // function would abort the process, so it is reported as an error instead.
    let build_result = catch_panic(
        || CompleteSailModelImpl::new_from_string(&setup_str)
    );

    match build_result {
        Some(Ok(model)) => Box::into_raw(Box::new(model)) as *mut CompleteSailModel,
        Some(Err(error)) => {
            set_last_error(format!("complete_sail_model_new: {}", error));

            std::ptr::null_mut()
        },
        // The panic message is already stored as the last error
        None => std::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_drop(sail_model: *mut CompleteSailModel) {
    if !sail_model.is_null() {
        unsafe {
            let _ = Box::from_raw(sail_model as *mut CompleteSailModelImpl);
        }
    }
}

/// Query the model for the number of sails
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_get_number_of_sails(sail_model: *mut CompleteSailModel) -> i32 {
    if sail_model.is_null() {
        set_last_error("complete_sail_model_get_number_of_sails: the model pointer is null");

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    rust_model.get_number_of_sails() as i32
}

/// Query the model for the total number of span lines (control points) in all the sails.
///
/// This is the minimum number of points needed by
/// `complete_sail_model_apply_controller_based_on_freestream`.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_get_number_of_span_lines(
    sail_model: *mut CompleteSailModel
) -> i32 {
    if sail_model.is_null() {
        set_last_error("complete_sail_model_get_number_of_span_lines: the model pointer is null");

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    rust_model.lifting_line_simulation.line_force_model.nr_span_lines() as i32
}

/// Query the model for the number of points where the freestream velocity is needed.
///
/// This is the number of points returned by `complete_sail_model_freestream_velocity`.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_get_number_of_freestream_velocity_points(
    sail_model: *mut CompleteSailModel
) -> i32 {
    if sail_model.is_null() {
        set_last_error(
            "complete_sail_model_get_number_of_freestream_velocity_points: \
             the model pointer is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    rust_model.lifting_line_simulation.get_freestream_velocity_points().len() as i32
}

/// Runs multiple steady state simulations with different controller loadings, and returns the
/// result with the highest effective power. That is, the power delivered by the thrust, minus the
/// power that is used by the sails. The number of loadings that are tested is defined by the
/// model settings.
///
/// Returns NULL if a pointer is null.
///
/// # Safety
/// - `wind_condition` must be a pointer returned by one of the `wind_condition_new_*` functions
/// - The returned pointer must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_optimal_steady_state_condition(
    sail_model: *mut CompleteSailModel,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    max_loading: f64,
) -> *mut SimulationResult {
    if sail_model.is_null() || wind_condition.is_null() {
        set_last_error(
            "complete_sail_model_simulate_optimal_steady_state_condition: \
             one of the input pointers is null"
        );

        return std::ptr::null_mut();
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let result = rust_model.simulate_optimal_steady_state_condition(
        wind_condition_rust,
        ship_velocity,
        max_loading
    );

    box_simulation_result(result)
}

/// Applies the controller and simulates a single steady state condition.
///
/// `controller_loading` defines how close to the max angle of attack/max spin ratio/max flap
/// angle/max suction rate the sails should be operated at. 1.0 means max values, while 0.0 means
/// sails in neutral position.
///
/// Returns NULL if a pointer is null.
///
/// # Safety
/// - `wind_condition` must be a pointer returned by one of the `wind_condition_new_*` functions
/// - The returned pointer must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_steady_state_condition(
    sail_model: *mut CompleteSailModel,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    controller_loading: f64,
) -> *mut SimulationResult {
    if sail_model.is_null() || wind_condition.is_null() {
        set_last_error(
            "complete_sail_model_simulate_steady_state_condition: \
             one of the input pointers is null"
        );

        return std::ptr::null_mut();
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let result = rust_model.simulate_steady_state_condition(
        wind_condition_rust,
        ship_velocity,
        controller_loading
    );

    box_simulation_result(result)
}

/// Same as `complete_sail_model_simulate_steady_state_condition`, but the result is simplified to
/// the total force and moment and the input power for each sail.
///
/// Returns the number of sails written to `results_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `results_out_length` is zero.
///
/// # Safety
/// - `results_out` must point to an array with room for at least `results_out_length` elements.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_steady_state_condition_simple_output(
    sail_model: *mut CompleteSailModel,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    controller_loading: f64,
    results_out: *mut SingleSailResult,
    results_out_length: usize,
) -> i32 {
    // Check for null pointer
    if sail_model.is_null() || wind_condition.is_null() || results_out.is_null() {
        set_last_error(
            "complete_sail_model_simulate_steady_state_condition_simple_output: \
             one of the input pointers is null"
        );

        return -1;
    }

    if results_out_length == 0 {
        set_last_error(
            "complete_sail_model_simulate_steady_state_condition_simple_output: \
             the output array has zero length"
        );

        return -2;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let rust_results = rust_model.simulate_steady_state_condition_simple_output(
        wind_condition_rust,
        ship_velocity,
        controller_loading
    );

    write_single_sail_results(&rust_results, results_out, results_out_length)
}

/// Same as `complete_sail_model_simulate_optimal_steady_state_condition`, but the result is
/// simplified to the total force and moment and the input power for each sail.
///
/// Returns the number of sails written to `results_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `results_out_length` is zero.
///
/// # Safety
/// - `results_out` must point to an array with room for at least `results_out_length` elements.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_optimal_steady_state_condition_simple_output(
    sail_model: *mut CompleteSailModel,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    max_loading: f64,
    results_out: *mut SingleSailResult,
    results_out_length: usize,
) -> i32 {
    if sail_model.is_null() || wind_condition.is_null() || results_out.is_null() {
        set_last_error(
            "complete_sail_model_simulate_optimal_steady_state_condition_simple_output: \
             one of the input pointers is null"
        );

        return -1;
    }

    if results_out_length == 0 {
        set_last_error(
            "complete_sail_model_simulate_optimal_steady_state_condition_simple_output: \
             the output array has zero length"
        );

        return -2;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let rust_results = rust_model.simulate_optimal_steady_state_condition_simple_output(
        wind_condition_rust,
        ship_velocity,
        max_loading
    );

    write_single_sail_results(&rust_results, results_out, results_out_length)
}

/// Runs an unsteady simulation from time zero until `end_time`, with a fixed `time_step`, and
/// stores a result for each time step in `results_out`.
///
/// Returns the number of results written to `results_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `results_out_length` is zero. Results that do not fit in
/// `results_out` are discarded, and entries beyond the returned count are left untouched.
///
/// # Safety
/// - `results_out` must point to an array with room for at least `results_out_length` pointers.
/// - Each of the returned result pointers must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_do_multiple_steps(
    sail_model: *mut CompleteSailModel,
    end_time: f64,
    time_step: f64,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    results_out: *mut *mut SimulationResult,
    results_out_length: usize,
) -> i32 {
    if sail_model.is_null() || wind_condition.is_null() || results_out.is_null() {
        set_last_error("complete_sail_model_do_multiple_steps: one of the input pointers is null");

        return -1;
    }

    if results_out_length == 0 {
        set_last_error("complete_sail_model_do_multiple_steps: the output array has zero length");

        return -2;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let rust_results = rust_model.do_multiple_steps(
        end_time,
        time_step,
        wind_condition_rust,
        ship_velocity
    );

    let actual_count = rust_results.len().min(results_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(results_out, actual_count)
    };

    for (i, rust_result) in rust_results.into_iter().take(actual_count).enumerate() {
        output_slice[i] = box_simulation_result(rust_result);
    }

    actual_count as i32
}

/// Returns the forces on the sails for a single time step.
///
/// Returns NULL if a pointer is null.
///
/// # Safety
/// - `wind_condition` must be a pointer returned by one of the `wind_condition_new_*` functions
/// - The returned pointer must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_do_step(
    sail_model: *mut CompleteSailModel,
    current_time: f64,
    time_step: f64,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
) -> *mut SimulationResult {
    if sail_model.is_null() || wind_condition.is_null() {
        set_last_error("complete_sail_model_do_step: one of the input pointers is null");

        return std::ptr::null_mut();
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let result = rust_model.do_step(
        current_time,
        time_step,
        wind_condition_rust,
        ship_velocity
    );

    box_simulation_result(result)
}

/// Updates the control parameters of the sails based on the wind condition alone, meaning that
/// the lift induced velocity is neglected.
///
/// Returns 0 on success, or -1 if a pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_apply_controller_based_on_wind_condition(
    sail_model: *mut CompleteSailModel,
    current_time: f64,
    time_step: f64,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    controller_loading: f64,
) -> i32 {
    if sail_model.is_null() || wind_condition.is_null() {
        set_last_error(
            "complete_sail_model_apply_controller_based_on_wind_condition: \
             one of the input pointers is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    rust_model.apply_controller_based_on_wind_condition(
        current_time,
        time_step,
        wind_condition_rust,
        ship_velocity,
        controller_loading
    );

    0
}

/// The freestream velocity at all the points where it is needed by the simulation. That is, the
/// apparent wind, including the effect of the ship velocity and inflow corrections.
///
/// The velocities are written to `velocity_out` as 3 doubles per point. The number of points is
/// given by `complete_sail_model_get_number_of_freestream_velocity_points`.
///
/// Returns the number of points written to `velocity_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `nr_points` is zero.
///
/// # Safety
/// - `velocity_out` must point to an array with room for at least `3 * nr_points` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_freestream_velocity(
    sail_model: *mut CompleteSailModel,
    wind_condition: *const WindCondition,
    ship_velocity: f64,
    time: f64,
    velocity_out: *mut f64,
    nr_points: usize,
) -> i32 {
    if sail_model.is_null() || wind_condition.is_null() || velocity_out.is_null() {
        set_last_error("complete_sail_model_freestream_velocity: one of the input pointers is null");

        return -1;
    }

    if nr_points == 0 {
        set_last_error("complete_sail_model_freestream_velocity: the output array has zero length");

        return -2;
    }

    let rust_model = unsafe {
        &*(sail_model as *const CompleteSailModelImpl)
    };

    let wind_condition_rust = unsafe { wind_condition_ref(wind_condition) };

    let freestream_velocity = rust_model.freestream_velocity(
        wind_condition_rust,
        ship_velocity,
        time
    );

    let actual_count = freestream_velocity.len().min(nr_points);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(velocity_out, 3 * actual_count)
    };

    for (i, velocity) in freestream_velocity.iter().take(actual_count).enumerate() {
        output_slice[3 * i..3 * i + 3].copy_from_slice(&velocity.0);
    }

    actual_count as i32
}

/// Updates the control parameters of the sails based on a freestream velocity field, given as 3
/// doubles per point.
///
/// `nr_points` must be at least the number of span lines in the model, which is given by
/// `complete_sail_model_get_number_of_span_lines`.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if `nr_points` is
/// smaller than the number of span lines.
///
/// # Safety
/// - `freestream_velocity` must point to an array with at least `3 * nr_points` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_apply_controller_based_on_freestream(
    sail_model: *mut CompleteSailModel,
    current_time: f64,
    time_step: f64,
    loading: f64,
    freestream_velocity: *const f64,
    nr_points: usize,
) -> i32 {
    if sail_model.is_null() || freestream_velocity.is_null() {
        set_last_error(
            "complete_sail_model_apply_controller_based_on_freestream: \
             one of the input pointers is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let nr_span_lines = rust_model.lifting_line_simulation.line_force_model.nr_span_lines();

    if nr_points < nr_span_lines {
        set_last_error(
            format!(
                "complete_sail_model_apply_controller_based_on_freestream: got {} velocity \
                 points, but the model has {} span lines",
                nr_points,
                nr_span_lines
            )
        );

        return -2;
    }

    let input_slice = unsafe {
        std::slice::from_raw_parts(freestream_velocity, 3 * nr_points)
    };

    let freestream_velocity_rust: Vec<SpatialVector> = input_slice.chunks_exact(3).map(
        |v| SpatialVector::from([v[0], v[1], v[2]])
    ).collect();

    rust_model.apply_controller_based_on_freestream(
        current_time,
        time_step,
        loading,
        &freestream_velocity_rust
    );

    0
}

/// Updates the control parameters of the sails based on the result of a previous simulation step.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `simulation_result` must be a pointer returned by one of the simulation functions.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_apply_controller_based_on_simulation_result(
    sail_model: *mut CompleteSailModel,
    current_time: f64,
    time_step: f64,
    loading: f64,
    simulation_result: *const SimulationResult,
) -> i32 {
    if sail_model.is_null() || simulation_result.is_null() {
        set_last_error(
            "complete_sail_model_apply_controller_based_on_simulation_result: \
             one of the input pointers is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    let rust_result = unsafe {
        &*(simulation_result as *const SimulationResultImpl)
    };

    rust_model.apply_controller_based_on_simulation_result(
        current_time,
        time_step,
        loading,
        rust_result
    );

    0
}

/// The current rotation angle of each sail around its span axis, in radians. There is one angle
/// per sail, and the number of sails is given by `complete_sail_model_get_number_of_sails`.
///
/// Returns the number of angles written to `angles_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `angles_out_length` is zero.
///
/// # Safety
/// - `angles_out` must point to an array with room for at least `angles_out_length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_local_wing_angles(
    sail_model: *mut CompleteSailModel,
    angles_out: *mut f64,
    angles_out_length: usize,
) -> i32 {
    if sail_model.is_null() || angles_out.is_null() {
        set_last_error("complete_sail_model_local_wing_angles: one of the input pointers is null");

        return -1;
    }

    if angles_out_length == 0 {
        set_last_error("complete_sail_model_local_wing_angles: the output array has zero length");

        return -2;
    }

    let rust_model = unsafe {
        &*(sail_model as *const CompleteSailModelImpl)
    };

    let angles = &rust_model.lifting_line_simulation.line_force_model.local_wing_angles;

    write_floats(angles, angles_out, angles_out_length)
}

/// Sets the rotation angle of each sail around its span axis, in radians. The number of values
/// must match the number of sails, given by `complete_sail_model_get_number_of_sails`.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if the number of
/// values does not match the number of sails.
///
/// # Safety
/// - `local_wing_angles` must point to an array with at least `length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_set_local_wing_angles(
    sail_model: *mut CompleteSailModel,
    local_wing_angles: *const f64,
    length: usize,
) -> i32 {
    if sail_model.is_null() || local_wing_angles.is_null() {
        set_last_error(
            "complete_sail_model_set_local_wing_angles: one of the input pointers is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    if length != rust_model.get_number_of_sails() {
        set_last_error(
            format!(
                "complete_sail_model_set_local_wing_angles: got {} angles, but the model has {} \
                 sails",
                length,
                rust_model.get_number_of_sails()
            )
        );

        return -2;
    }

    let input_slice = unsafe {
        std::slice::from_raw_parts(local_wing_angles, length)
    };

    rust_model.set_local_wing_angles(input_slice);

    0
}

/// The internal state of the section model of each sail. The meaning depends on the section model
/// that is used, and is for instance the revolutions per second for a rotating cylinder. There is
/// one value per sail.
///
/// Returns the number of values written to `internal_state_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `internal_state_out_length` is zero.
///
/// # Safety
/// - `internal_state_out` must point to an array with room for at least
///   `internal_state_out_length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_section_models_internal_state(
    sail_model: *mut CompleteSailModel,
    internal_state_out: *mut f64,
    internal_state_out_length: usize,
) -> i32 {
    if sail_model.is_null() || internal_state_out.is_null() {
        set_last_error(
            "complete_sail_model_section_models_internal_state: \
             one of the input pointers is null"
        );

        return -1;
    }

    if internal_state_out_length == 0 {
        set_last_error(
            "complete_sail_model_section_models_internal_state: \
             the output array has zero length"
        );

        return -2;
    }

    let rust_model = unsafe {
        &*(sail_model as *const CompleteSailModelImpl)
    };

    let internal_state = rust_model
        .lifting_line_simulation
        .line_force_model
        .section_models_internal_state();

    write_floats(&internal_state, internal_state_out, internal_state_out_length)
}

/// Sets the internal state of the section model of each sail. The number of values must match the
/// number of sails, given by `complete_sail_model_get_number_of_sails`.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if the number of
/// values does not match the number of sails.
///
/// # Safety
/// - `internal_state` must point to an array with at least `length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_set_section_models_internal_state(
    sail_model: *mut CompleteSailModel,
    internal_state: *const f64,
    length: usize,
) -> i32 {
    if sail_model.is_null() || internal_state.is_null() {
        set_last_error(
            "complete_sail_model_set_section_models_internal_state: \
             one of the input pointers is null"
        );

        return -1;
    }

    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };

    if length != rust_model.get_number_of_sails() {
        set_last_error(
            format!(
                "complete_sail_model_set_section_models_internal_state: got {} values, but the \
                 model has {} sails",
                length,
                rust_model.get_number_of_sails()
            )
        );

        return -2;
    }

    let input_slice = unsafe {
        std::slice::from_raw_parts(internal_state, length)
    };

    rust_model.set_section_models_internal_state(input_slice);

    0
}

/// Helper that copies simplified results into a caller owned array.
fn write_single_sail_results(
    rust_results: &[stormbird::common_utils::results::simplfied::SingleSailResult],
    results_out: *mut SingleSailResult,
    results_out_length: usize,
) -> i32 {
    let actual_count = rust_results.len().min(results_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(results_out, actual_count)
    };

    for (i, rust_result) in rust_results.iter().take(actual_count).enumerate() {
        output_slice[i] = SingleSailResult::from(rust_result.clone());
    }

    actual_count as i32
}

/// Helper that copies a vector of floats into a caller owned array.
fn write_floats(values: &[f64], values_out: *mut f64, values_out_length: usize) -> i32 {
    let actual_count = values.len().min(values_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(values_out, actual_count)
    };

    output_slice.copy_from_slice(&values[0..actual_count]);

    actual_count as i32
}
