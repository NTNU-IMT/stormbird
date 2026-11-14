use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::set_last_error;
use crate::ffi_utils::{write_floats, write_vectors};

use stormbird::common_utils::results::simplfied::SingleSailResult as SingleSailResultImpl;
use stormbird::common_utils::results::simulation::SimulationResult as SimulationResultImpl;
use stormbird::common_utils::forces_and_moments::IntegratedValues as IntegratedValuesImpl;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SingleSailResult {
    pub force: [f64; 3],
    pub moment: [f64; 3],
    pub input_power: f64
}

impl From<SingleSailResultImpl> for SingleSailResult {
    fn from(r: SingleSailResultImpl) -> Self {
        Self {
            force: r.force.into(),
            moment: r.moment.into(),
            input_power: r.input_power
        }
    }
}

/// Opaque pointer structure to the full SimulationResult.
///
/// The full result contains nested and variable sized data that is not practical to expose
/// directly over a C-ABI. It is therefore handled as an opaque handle, with accessor functions
/// for the most important quantities. A handle must always be released with
/// `simulation_result_drop`.
#[repr(C)]
pub struct SimulationResult {
    _private: [u8; 0],
}

/// Wraps a simulation result from the rust library in a heap allocation owned by the caller.
pub(crate) fn box_simulation_result(result: SimulationResultImpl) -> *mut SimulationResult {
    Box::into_raw(Box::new(result)) as *mut SimulationResult
}

/// Frees a simulation result returned by any of the simulation functions.
///
/// # Safety
/// - `result` must be a pointer returned by this library, and must not be used afterwards.
/// - Calling this function with a null pointer is allowed, and does nothing.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_drop(result: *mut SimulationResult) {
    if !result.is_null() {
        unsafe {
            let _ = Box::from_raw(result as *mut SimulationResultImpl);
        }
    }
}

/// The simulation time the result belongs to. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_time(result: *const SimulationResult) -> f64 {
    if result.is_null() {
        set_last_error("simulation_result_time: the result pointer is null");

        return f64::NAN;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    rust_result.time
}

/// The number of sails (wings) the result contains data for. Returns -1 if the input pointer is
/// null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_nr_of_wings(result: *const SimulationResult) -> i32 {
    if result.is_null() {
        set_last_error("simulation_result_nr_of_wings: the result pointer is null");

        return -1;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    rust_result.nr_of_wings() as i32
}

/// Simplifies the result to the total force, total moment and input power for each sail.
///
/// Returns the number of sails written to `results_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `results_out_length` is zero.
///
/// # Safety
/// - `results_out` must point to an array with room for at least `results_out_length` elements.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_as_simplified(
    result: *const SimulationResult,
    results_out: *mut SingleSailResult,
    results_out_length: usize,
) -> i32 {
    if result.is_null() || results_out.is_null() {
        set_last_error("simulation_result_as_simplified: one of the input pointers is null");

        return -1;
    }

    if results_out_length == 0 {
        set_last_error("simulation_result_as_simplified: the output array has zero length");

        return -2;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    let simplified = rust_result.as_simplified();

    let actual_count = simplified.len().min(results_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(results_out, actual_count)
    };

    for (i, single_result) in simplified.iter().take(actual_count).enumerate() {
        output_slice[i] = SingleSailResult::from(single_result.clone());
    }

    actual_count as i32
}

/// The sum of the integrated forces on all the sails, written as 3 doubles to `force_out`.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `force_out` must point to an array with room for at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_integrated_forces_sum(
    result: *const SimulationResult,
    force_out: *mut f64,
) -> i32 {
    if result.is_null() || force_out.is_null() {
        set_last_error(
            "simulation_result_integrated_forces_sum: one of the input pointers is null"
        );

        return -1;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    let force = rust_result.integrated_forces_sum();

    let output_slice = unsafe { std::slice::from_raw_parts_mut(force_out, 3) };

    output_slice.copy_from_slice(&force.0);

    0
}

/// The sum of the integrated moments on all the sails, written as 3 doubles to `moment_out`.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `moment_out` must point to an array with room for at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_integrated_moments_sum(
    result: *const SimulationResult,
    moment_out: *mut f64,
) -> i32 {
    if result.is_null() || moment_out.is_null() {
        set_last_error(
            "simulation_result_integrated_moments_sum: one of the input pointers is null"
        );

        return -1;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    let moment = rust_result.integrated_moments_sum();

    let output_slice = unsafe { std::slice::from_raw_parts_mut(moment_out, 3) };

    output_slice.copy_from_slice(&moment.0);

    0
}

/// The sum of the input power used by all the sails. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_input_power_sum(result: *const SimulationResult) -> f64 {
    if result.is_null() {
        set_last_error("simulation_result_input_power_sum: the result pointer is null");

        return f64::NAN;
    }

    let rust_result = unsafe { &*(result as *const SimulationResultImpl) };

    rust_result.input_power_sum()
}

/// Values integrated over a sail, split into the different contributions, as 3 doubles for each.
/// Corresponds to the `IntegratedValues` class in pystormbird.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct IntegratedValues {
    pub circulatory: [f64; 3],
    pub sectional_drag: [f64; 3],
    pub added_mass: [f64; 3],
    pub gyroscopic: [f64; 3],
    pub total: [f64; 3],
}

impl From<&IntegratedValuesImpl> for IntegratedValues {
    fn from(values: &IntegratedValuesImpl) -> Self {
        Self {
            circulatory: values.circulatory.0,
            sectional_drag: values.sectional_drag.0,
            added_mass: values.added_mass.0,
            gyroscopic: values.gyroscopic.0,
            total: values.total.0,
        }
    }
}

/// The forces on a single line element, split into the different contributions, as 3 doubles for
/// each. Corresponds to one element in the `SectionalForces` class in pystormbird.
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SectionalForces {
    pub circulatory: [f64; 3],
    pub sectional_drag: [f64; 3],
    pub added_mass: [f64; 3],
    pub gyroscopic_moment: [f64; 3],
    pub total: [f64; 3],
}

/// Returns a reference to the result behind the opaque pointer.
///
/// # Safety
/// `result` must be a non-null pointer returned by this library.
unsafe fn result_ref<'a>(result: *const SimulationResult) -> &'a SimulationResultImpl {
    unsafe { &*(result as *const SimulationResultImpl) }
}

/// Reads a simulation result from a JSON file, e.g., as written by the actuator line model.
///
/// Returns NULL on error, in which case `stormbird_last_error_message` describes what went wrong,
/// e.g., that the file could not be read or that the content could not be parsed.
///
/// # Safety
/// - `file_path` must be a valid null-terminated C string
/// - The returned pointer must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_new_from_json_file(
    file_path: *const c_char
) -> *mut SimulationResult {
    if file_path.is_null() {
        set_last_error("simulation_result_new_from_json_file: the file path pointer is null");

        return std::ptr::null_mut();
    }

    let file_path = match unsafe { CStr::from_ptr(file_path) }.to_str() {
        Ok(file_path) => file_path,
        Err(error) => {
            set_last_error(format!(
                "simulation_result_new_from_json_file: the file path is not valid UTF-8: {}",
                error
            ));

            return std::ptr::null_mut();
        }
    };

    match SimulationResultImpl::from_json(file_path) {
        Ok(result) => box_simulation_result(result),
        Err(error) => {
            set_last_error(format!(
                "simulation_result_new_from_json_file: failed to read the result from {}: {}",
                file_path, error
            ));

            std::ptr::null_mut()
        }
    }
}

/// The number of line elements (span lines) in the result. Returns -1 if the pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_nr_of_span_lines(result: *const SimulationResult) -> i32 {
    if result.is_null() {
        set_last_error("simulation_result_nr_of_span_lines: the result pointer is null");

        return -1;
    }

    unsafe { result_ref(result) }.nr_span_lines() as i32
}

/// The number of iterations used by the solver. Returns -1 if the pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_iterations(result: *const SimulationResult) -> i32 {
    if result.is_null() {
        set_last_error("simulation_result_iterations: the result pointer is null");

        return -1;
    }

    unsafe { result_ref(result) }.iterations as i32
}

/// The residual of the final solution. Returns NaN if the pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_residual(result: *const SimulationResult) -> f64 {
    if result.is_null() {
        set_last_error("simulation_result_residual: the result pointer is null");

        return f64::NAN;
    }

    unsafe { result_ref(result) }.residual
}

/// The indices of the line elements that belong to each sail, written as 2 values per sail to
/// `indices_out`: the index of the first line element, and one past the index of the last.
///
/// Returns the number of sails written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_wings` is zero.
///
/// # Safety
/// - `indices_out` must point to an array with room for at least `2 * nr_wings` values.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_wing_indices(
    result: *const SimulationResult,
    indices_out: *mut usize,
    nr_wings: usize,
) -> i32 {
    if result.is_null() || indices_out.is_null() {
        set_last_error("simulation_result_wing_indices: one of the input pointers is null");

        return -1;
    }

    if nr_wings == 0 {
        set_last_error("simulation_result_wing_indices: the output array has zero length");

        return -2;
    }

    let wing_indices = &unsafe { result_ref(result) }.wing_indices;

    let actual_count = wing_indices.len().min(nr_wings);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(indices_out, 2 * actual_count) };

    for (i, range) in wing_indices.iter().take(actual_count).enumerate() {
        output_slice[2 * i] = range.start;
        output_slice[2 * i + 1] = range.end;
    }

    actual_count as i32
}

/// Checks the pointers and the output length for the functions that write arrays. Returns the
/// error code to return, if any.
fn check_output(function_name: &str, result: *const SimulationResult, out_is_null: bool, length: usize) -> Option<i32> {
    if result.is_null() || out_is_null {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return Some(-1);
    }

    if length == 0 {
        set_last_error(format!("{}: the output array has zero length", function_name));

        return Some(-2);
    }

    None
}

/// The control points of the line elements, written as 3 doubles per point to `points_out`.
///
/// Returns the number of points written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_points` is zero.
///
/// # Safety
/// - `points_out` must point to an array with room for at least `3 * nr_points` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_ctrl_points(
    result: *const SimulationResult,
    points_out: *mut f64,
    nr_points: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_ctrl_points", result, points_out.is_null(), nr_points
    ) {
        return code;
    }

    write_vectors(&unsafe { result_ref(result) }.ctrl_points, points_out, nr_points)
}

/// The forces integrated over each sail, written as one `IntegratedValues` per sail.
///
/// Returns the number of sails written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_wings` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_wings` elements.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_integrated_forces(
    result: *const SimulationResult,
    values_out: *mut IntegratedValues,
    nr_wings: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_integrated_forces", result, values_out.is_null(), nr_wings
    ) {
        return code;
    }

    write_integrated_values(&unsafe { result_ref(result) }.integrated_forces, values_out, nr_wings)
}

/// The moments integrated over each sail, written as one `IntegratedValues` per sail.
///
/// Returns the number of sails written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_wings` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_wings` elements.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_integrated_moments(
    result: *const SimulationResult,
    values_out: *mut IntegratedValues,
    nr_wings: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_integrated_moments", result, values_out.is_null(), nr_wings
    ) {
        return code;
    }

    write_integrated_values(&unsafe { result_ref(result) }.integrated_moments, values_out, nr_wings)
}

fn write_integrated_values(
    values: &[IntegratedValuesImpl],
    values_out: *mut IntegratedValues,
    length: usize
) -> i32 {
    let actual_count = values.len().min(length);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(values_out, actual_count) };

    for (i, value) in values.iter().take(actual_count).enumerate() {
        output_slice[i] = IntegratedValues::from(value);
    }

    actual_count as i32
}

/// The input power used by each sail.
///
/// Returns the number of values written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_wings` is zero.
///
/// # Safety
/// - `input_power_out` must point to an array with room for at least `nr_wings` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_input_power(
    result: *const SimulationResult,
    input_power_out: *mut f64,
    nr_wings: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_input_power", result, input_power_out.is_null(), nr_wings
    ) {
        return code;
    }

    write_floats(&unsafe { result_ref(result) }.input_power, input_power_out, nr_wings)
}

/// The forces on each line element, written as one `SectionalForces` per line element.
///
/// Returns the number of line elements written, or a negative error code: -1 if a pointer is null,
/// -2 if `nr_span_lines` is zero.
///
/// # Safety
/// - `forces_out` must point to an array with room for at least `nr_span_lines` elements.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_sectional_forces(
    result: *const SimulationResult,
    forces_out: *mut SectionalForces,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_sectional_forces", result, forces_out.is_null(), nr_span_lines
    ) {
        return code;
    }

    let forces = &unsafe { result_ref(result) }.sectional_forces;

    let actual_count = forces.total.len().min(nr_span_lines);

    let output_slice = unsafe { std::slice::from_raw_parts_mut(forces_out, actual_count) };

    for (i, output) in output_slice.iter_mut().enumerate() {
        *output = SectionalForces {
            circulatory: forces.circulatory[i].0,
            sectional_drag: forces.sectional_drag[i].0,
            added_mass: forces.added_mass[i].0,
            gyroscopic_moment: forces.gyroscopic_moment[i].0,
            total: forces.total[i].0,
        };
    }

    actual_count as i32
}

/// The circulation strength on each line element.
///
/// Returns the number of values written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_span_lines` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_circulation_strength(
    result: *const SimulationResult,
    values_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_circulation_strength", result, values_out.is_null(),
        nr_span_lines
    ) {
        return code;
    }

    write_floats(
        &unsafe { result_ref(result) }.force_input.circulation_strength, values_out, nr_span_lines
    )
}

/// The velocity used to compute the forces on each line element, including the induced
/// velocities and the motion of the sails, written as 3 doubles per line element.
///
/// Returns the number of line elements written, or a negative error code: -1 if a pointer is null,
/// -2 if `nr_span_lines` is zero.
///
/// # Safety
/// - `velocity_out` must point to an array with room for at least `3 * nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_velocity(
    result: *const SimulationResult,
    velocity_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_velocity", result, velocity_out.is_null(), nr_span_lines
    ) {
        return code;
    }

    write_vectors(&unsafe { result_ref(result) }.force_input.velocity, velocity_out, nr_span_lines)
}

/// The angle of attack on each line element, in radians.
///
/// Returns the number of values written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_span_lines` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_angles_of_attack(
    result: *const SimulationResult,
    values_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_angles_of_attack", result, values_out.is_null(),
        nr_span_lines
    ) {
        return code;
    }

    write_floats(
        &unsafe { result_ref(result) }.force_input.angles_of_attack, values_out, nr_span_lines
    )
}

/// The internal state given to the section model on each line element, e.g., the spin ratio for
/// a rotating cylinder.
///
/// Returns the number of values written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_span_lines` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_section_models_internal_state(
    result: *const SimulationResult,
    values_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_section_models_internal_state", result,
        values_out.is_null(), nr_span_lines
    ) {
        return code;
    }

    write_floats(
        &unsafe { result_ref(result) }.force_input.section_models_internal_state,
        values_out,
        nr_span_lines
    )
}

/// The amount of stall used for the lift and drag on each line element, between 0 (no stall) and
/// 1 (full stall).
///
/// Returns the number of values written, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_span_lines` is zero.
///
/// # Safety
/// - `values_out` must point to an array with room for at least `nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_amount_of_stall(
    result: *const SimulationResult,
    values_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_amount_of_stall", result, values_out.is_null(),
        nr_span_lines
    ) {
        return code;
    }

    write_floats(
        &unsafe { result_ref(result) }.force_input.amount_of_stall, values_out, nr_span_lines
    )
}

/// The acceleration of each line element, written as 3 doubles per line element.
///
/// Returns the number of line elements written, or a negative error code: -1 if a pointer is null,
/// -2 if `nr_span_lines` is zero.
///
/// # Safety
/// - `acceleration_out` must point to an array with room for at least `3 * nr_span_lines` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_acceleration(
    result: *const SimulationResult,
    acceleration_out: *mut f64,
    nr_span_lines: usize,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_acceleration", result, acceleration_out.is_null(),
        nr_span_lines
    ) {
        return code;
    }

    write_vectors(
        &unsafe { result_ref(result) }.force_input.acceleration, acceleration_out, nr_span_lines
    )
}

/// The angular velocity of the sails, written as 3 doubles to `rotation_velocity_out`.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `rotation_velocity_out` must point to an array with room for at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn simulation_result_force_input_rotation_velocity(
    result: *const SimulationResult,
    rotation_velocity_out: *mut f64,
) -> i32 {
    if let Some(code) = check_output(
        "simulation_result_force_input_rotation_velocity", result,
        rotation_velocity_out.is_null(), 1
    ) {
        return code;
    }

    write_vectors(
        &[unsafe { result_ref(result) }.force_input.rotation_velocity], rotation_velocity_out, 1
    );

    0
}
