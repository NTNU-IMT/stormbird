use crate::error::set_last_error;

use stormbird::common_utils::results::simplfied::SingleSailResult as SingleSailResultImpl;
use stormbird::common_utils::results::simulation::SimulationResult as SimulationResultImpl;

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
