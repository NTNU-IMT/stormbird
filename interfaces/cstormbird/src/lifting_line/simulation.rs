// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to a dynamic simulation using a lifting line model, without the additional
//! functionality of the complete sail model.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::{
    catch_panic,
    set_last_error,
};
use crate::results::{
    SimulationResult,
    box_simulation_result,
};

use stormbird::lifting_line::simulation::Simulation as SimulationImpl;
use stormbird::lifting_line::wake::WakeData;

use stormath::spatial_vector::SpatialVector;

/// Opaque pointer structure to a lifting line Simulation
#[repr(C)]
pub struct LiftingLineSimulation {
    _private: [u8; 0],
}

/// Creates a new lifting line simulation from a JSON setup string.
///
/// Returns NULL on error, in which case `stormbird_last_error_message` describes what went wrong.
///
/// # Safety
/// - `setup_string` must be a valid null-terminated C string
/// - The returned pointer must be freed with `lifting_line_simulation_drop`
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_new(
    setup_string: *const c_char
) -> *mut LiftingLineSimulation {
    if setup_string.is_null() {
        set_last_error("lifting_line_simulation_new: the setup string pointer is null");

        return std::ptr::null_mut();
    }

    let setup_str = match unsafe { CStr::from_ptr(setup_string) }.to_str() {
        Ok(s) => s.to_string(),
        Err(error) => {
            set_last_error(
                format!(
                    "lifting_line_simulation_new: the setup string is not valid UTF-8: {}",
                    error
                )
            );

            return std::ptr::null_mut();
        }
    };

    // Building a simulation from a valid setup string can still panic on inconsistent input, and
    // a panic that unwinds out of this function would abort the process.
    let build_result = catch_panic(|| SimulationImpl::new_from_string(&setup_str));

    match build_result {
        Some(Ok(simulation)) => {
            Box::into_raw(Box::new(simulation)) as *mut LiftingLineSimulation
        },
        Some(Err(error)) => {
            set_last_error(format!("lifting_line_simulation_new: {}", error));

            std::ptr::null_mut()
        },
        // The panic message is already stored as the last error
        None => std::ptr::null_mut()
    }
}

/// Frees a simulation returned by `lifting_line_simulation_new`.
///
/// # Safety
/// - `simulation` must be a pointer returned by this library, and must not be used afterwards.
/// - Calling this function with a null pointer is allowed, and does nothing.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_drop(simulation: *mut LiftingLineSimulation) {
    if !simulation.is_null() {
        unsafe {
            let _ = Box::from_raw(simulation as *mut SimulationImpl);
        }
    }
}

/// Query the simulation for the number of sails (wings).
///
/// This is the number of values used by the local wing angles and the internal states of the
/// section models.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_get_number_of_sails(
    simulation: *mut LiftingLineSimulation
) -> i32 {
    match simulation_ref(simulation, "lifting_line_simulation_get_number_of_sails") {
        Some(rust_simulation) => rust_simulation.line_force_model.nr_wings() as i32,
        None => -1
    }
}

/// Query the simulation for the total number of span lines (control points) in all the sails.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_get_number_of_span_lines(
    simulation: *mut LiftingLineSimulation
) -> i32 {
    match simulation_ref(simulation, "lifting_line_simulation_get_number_of_span_lines") {
        Some(rust_simulation) => rust_simulation.line_force_model.nr_span_lines() as i32,
        None => -1
    }
}

/// Query the simulation for the number of points where the freestream velocity must be specified
/// in order to run a time step.
///
/// This is the number of control points for a quasi-steady simulation, and the control points
/// plus all the wake points for a dynamic simulation. The number can change between time steps
/// while a dynamic wake is growing, so it should be queried before each `do_step` call.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_get_number_of_freestream_velocity_points(
    simulation: *mut LiftingLineSimulation
) -> i32 {
    let function_name = "lifting_line_simulation_get_number_of_freestream_velocity_points";

    match simulation_ref(simulation, function_name) {
        Some(rust_simulation) => rust_simulation.get_freestream_velocity_points().len() as i32,
        None => -1
    }
}

/// Whether the simulation uses a dynamic wake. Returns 1 for a dynamic wake, 0 for a quasi-steady
/// wake, and -1 if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_has_dynamic_wake(
    simulation: *mut LiftingLineSimulation
) -> i32 {
    match simulation_ref(simulation, "lifting_line_simulation_has_dynamic_wake") {
        Some(rust_simulation) => {
            match rust_simulation.wake_data {
                WakeData::Dynamic(_) => 1,
                WakeData::QuasiSteady(_) => 0
            }
        },
        None => -1
    }
}

/// The points where the freestream velocity must be specified in order to run a time step.
///
/// The points are written to `points_out` as 3 doubles per point. The number of points is given
/// by `lifting_line_simulation_get_number_of_freestream_velocity_points`.
///
/// Returns the number of points written to `points_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `nr_points` is zero.
///
/// # Safety
/// - `points_out` must point to an array with room for at least `3 * nr_points` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_get_freestream_velocity_points(
    simulation: *mut LiftingLineSimulation,
    points_out: *mut f64,
    nr_points: usize,
) -> i32 {
    let function_name = "lifting_line_simulation_get_freestream_velocity_points";

    if simulation.is_null() || points_out.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return -1;
    }

    if nr_points == 0 {
        set_last_error(format!("{}: the output array has zero length", function_name));

        return -2;
    }

    let rust_simulation = unsafe { &*(simulation as *const SimulationImpl) };

    let points = rust_simulation.get_freestream_velocity_points();

    write_vectors(&points, points_out, nr_points)
}

/// Sets the position of the sails, without changing the velocity.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `translation` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_translation_only(
    simulation: *mut LiftingLineSimulation,
    translation: *const f64,
) -> i32 {
    let function_name = "lifting_line_simulation_set_translation_only";

    let (rust_simulation, translation_vector) = match mutable_simulation_and_vector(
        simulation,
        translation,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.set_translation_only(translation_vector);

    0
}

/// Sets the orientation of the sails, without changing the velocity.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `rotation` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_rotation_only(
    simulation: *mut LiftingLineSimulation,
    rotation: *const f64,
) -> i32 {
    let function_name = "lifting_line_simulation_set_rotation_only";

    let (rust_simulation, rotation_vector) = match mutable_simulation_and_vector(
        simulation,
        rotation,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.set_rotation_only(rotation_vector);

    0
}

/// Sets both the position and the orientation of the sails, and computes the linear and angular
/// velocity from the change since the previous call, using a finite difference.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `translation` and `rotation` must both point to arrays with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_translation_and_rotation_with_finite_difference_for_the_velocity(
    simulation: *mut LiftingLineSimulation,
    time_step: f64,
    translation: *const f64,
    rotation: *const f64,
) -> i32 {
    let function_name =
        "lifting_line_simulation_set_translation_and_rotation_with_finite_difference_for_the_velocity";

    if simulation.is_null() || translation.is_null() || rotation.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return -1;
    }

    let rust_simulation = unsafe { &mut *(simulation as *mut SimulationImpl) };

    rust_simulation.line_force_model
        .set_translation_and_rotation_with_finite_difference_for_the_velocity(
            time_step,
            unsafe { vector_from_c(translation) },
            unsafe { vector_from_c(rotation) }
        );

    0
}

/// Sets the position of the sails, and computes the linear velocity from the change since the
/// previous call, using a finite difference.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `translation` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_translation_with_velocity_using_finite_difference(
    simulation: *mut LiftingLineSimulation,
    translation: *const f64,
    time_step: f64,
) -> i32 {
    let function_name =
        "lifting_line_simulation_set_translation_with_velocity_using_finite_difference";

    let (rust_simulation, translation_vector) = match mutable_simulation_and_vector(
        simulation,
        translation,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.rigid_body_motion
        .update_translation_with_velocity_using_finite_difference(
            translation_vector,
            time_step
        );

    rust_simulation.line_force_model.update_global_data_representations();

    0
}

/// Sets the orientation of the sails, and computes the angular velocity from the change since the
/// previous call, using a finite difference.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `rotation` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_rotation_with_velocity_using_finite_difference(
    simulation: *mut LiftingLineSimulation,
    rotation: *const f64,
    time_step: f64,
) -> i32 {
    let function_name =
        "lifting_line_simulation_set_rotation_with_velocity_using_finite_difference";

    let (rust_simulation, rotation_vector) = match mutable_simulation_and_vector(
        simulation,
        rotation,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.rigid_body_motion
        .update_rotation_with_velocity_using_finite_difference(
            rotation_vector,
            time_step
        );

    rust_simulation.line_force_model.update_global_data_representations();

    0
}

/// Sets the linear velocity of the sails directly, without changing the position.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `linear_velocity` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_velocity_linear(
    simulation: *mut LiftingLineSimulation,
    linear_velocity: *const f64,
) -> i32 {
    let function_name = "lifting_line_simulation_set_velocity_linear";

    let (rust_simulation, velocity_vector) = match mutable_simulation_and_vector(
        simulation,
        linear_velocity,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.rigid_body_motion.velocity_linear = velocity_vector;

    0
}

/// Sets the angular velocity of the sails directly, without changing the orientation.
///
/// Returns 0 on success, or -1 if a pointer is null.
///
/// # Safety
/// - `angular_velocity` must point to an array with at least 3 doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_velocity_angular(
    simulation: *mut LiftingLineSimulation,
    angular_velocity: *const f64,
) -> i32 {
    let function_name = "lifting_line_simulation_set_velocity_angular";

    let (rust_simulation, velocity_vector) = match mutable_simulation_and_vector(
        simulation,
        angular_velocity,
        function_name
    ) {
        Some(values) => values,
        None => return -1
    };

    rust_simulation.line_force_model.rigid_body_motion.velocity_angular = velocity_vector;

    0
}

/// The current rotation angle of each sail around its span axis, in radians. There is one angle
/// per sail, and the number of sails is given by `lifting_line_simulation_get_number_of_sails`.
///
/// Returns the number of angles written to `angles_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `angles_out_length` is zero.
///
/// # Safety
/// - `angles_out` must point to an array with room for at least `angles_out_length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_local_wing_angles(
    simulation: *mut LiftingLineSimulation,
    angles_out: *mut f64,
    angles_out_length: usize,
) -> i32 {
    let function_name = "lifting_line_simulation_local_wing_angles";

    if simulation.is_null() || angles_out.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return -1;
    }

    if angles_out_length == 0 {
        set_last_error(format!("{}: the output array has zero length", function_name));

        return -2;
    }

    let rust_simulation = unsafe { &*(simulation as *const SimulationImpl) };

    write_floats(
        &rust_simulation.line_force_model.local_wing_angles,
        angles_out,
        angles_out_length
    )
}

/// Sets the rotation angle of each sail around its span axis, in radians. The number of values
/// must match the number of sails, given by `lifting_line_simulation_get_number_of_sails`.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if the number of
/// values does not match the number of sails.
///
/// # Safety
/// - `local_wing_angles` must point to an array with at least `length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_local_wing_angles(
    simulation: *mut LiftingLineSimulation,
    local_wing_angles: *const f64,
    length: usize,
) -> i32 {
    let function_name = "lifting_line_simulation_set_local_wing_angles";

    let input_slice = match input_values_for_each_sail(
        simulation,
        local_wing_angles,
        length,
        "angles",
        function_name
    ) {
        Ok(slice) => slice,
        Err(code) => return code
    };

    let rust_simulation = unsafe { &mut *(simulation as *mut SimulationImpl) };

    rust_simulation.line_force_model.set_local_wing_angles(input_slice);

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
pub extern "C" fn lifting_line_simulation_section_models_internal_state(
    simulation: *mut LiftingLineSimulation,
    internal_state_out: *mut f64,
    internal_state_out_length: usize,
) -> i32 {
    let function_name = "lifting_line_simulation_section_models_internal_state";

    if simulation.is_null() || internal_state_out.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return -1;
    }

    if internal_state_out_length == 0 {
        set_last_error(format!("{}: the output array has zero length", function_name));

        return -2;
    }

    let rust_simulation = unsafe { &*(simulation as *const SimulationImpl) };

    let internal_state = rust_simulation.line_force_model.section_models_internal_state();

    write_floats(&internal_state, internal_state_out, internal_state_out_length)
}

/// Sets the internal state of the section model of each sail. The number of values must match the
/// number of sails, given by `lifting_line_simulation_get_number_of_sails`.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if the number of
/// values does not match the number of sails.
///
/// # Safety
/// - `internal_state` must point to an array with at least `length` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_set_section_models_internal_state(
    simulation: *mut LiftingLineSimulation,
    internal_state: *const f64,
    length: usize,
) -> i32 {
    let function_name = "lifting_line_simulation_set_section_models_internal_state";

    let input_slice = match input_values_for_each_sail(
        simulation,
        internal_state,
        length,
        "values",
        function_name
    ) {
        Ok(slice) => slice,
        Err(code) => return code
    };

    let rust_simulation = unsafe { &mut *(simulation as *mut SimulationImpl) };

    rust_simulation.line_force_model.set_section_models_internal_state(input_slice);

    0
}

/// Resets the circulation strength stored from the previous time step to zero.
///
/// Returns 0 on success, or -1 if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_reset_previous_circulation_strength(
    simulation: *mut LiftingLineSimulation
) -> i32 {
    let function_name = "lifting_line_simulation_reset_previous_circulation_strength";

    if simulation.is_null() {
        set_last_error(format!("{}: the simulation pointer is null", function_name));

        return -1;
    }

    let rust_simulation = unsafe { &mut *(simulation as *mut SimulationImpl) };

    let nr_sections = rust_simulation.line_force_model.nr_span_lines();

    rust_simulation.previous_circulation_strength = vec![0.0; nr_sections];

    0
}

/// Steps the simulation forward in time by one time step, and returns the forces on the sails.
///
/// `freestream_velocity` holds 3 doubles per point, for the points returned by
/// `lifting_line_simulation_get_freestream_velocity_points`, and `nr_points` must match that
/// number exactly.
///
/// Returns NULL on error, in which case `stormbird_last_error_message` describes what went wrong.
///
/// # Safety
/// - `freestream_velocity` must point to an array with at least `3 * nr_points` doubles.
/// - The returned pointer must be freed with `simulation_result_drop`
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_do_step(
    simulation: *mut LiftingLineSimulation,
    time: f64,
    time_step: f64,
    freestream_velocity: *const f64,
    nr_points: usize,
) -> *mut SimulationResult {
    let function_name = "lifting_line_simulation_do_step";

    if simulation.is_null() || freestream_velocity.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return std::ptr::null_mut();
    }

    let rust_simulation = unsafe { &mut *(simulation as *mut SimulationImpl) };

    let expected_nr_points = rust_simulation.get_freestream_velocity_points().len();

    if nr_points != expected_nr_points {
        set_last_error(
            format!(
                "{}: got the freestream velocity at {} points, but the simulation needs it at {} \
                 points",
                function_name,
                nr_points,
                expected_nr_points
            )
        );

        return std::ptr::null_mut();
    }

    let freestream_velocity_rust = unsafe { vectors_from_c(freestream_velocity, nr_points) };

    let result = rust_simulation.do_step(time, time_step, &freestream_velocity_rust);

    box_simulation_result(result)
}

/// The velocities induced by the wake at the given points, written as 3 doubles per point.
///
/// This is only available for simulations with a dynamic wake, which can be checked with
/// `lifting_line_simulation_has_dynamic_wake`.
///
/// Returns the number of points written to `velocity_out`, or a negative error code:
/// -1 if a pointer is null, -2 if `nr_points` is zero, -5 if the simulation uses a quasi-steady
/// wake.
///
/// # Safety
/// - `points` and `velocity_out` must both point to arrays with at least `3 * nr_points` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn lifting_line_simulation_induced_velocities(
    simulation: *mut LiftingLineSimulation,
    points: *const f64,
    nr_points: usize,
    velocity_out: *mut f64,
) -> i32 {
    let function_name = "lifting_line_simulation_induced_velocities";

    if simulation.is_null() || points.is_null() || velocity_out.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return -1;
    }

    if nr_points == 0 {
        set_last_error(format!("{}: no points were given", function_name));

        return -2;
    }

    let rust_simulation = unsafe { &*(simulation as *const SimulationImpl) };

    // The rust function panics for quasi-steady wakes, where the induced velocities are not
    // available, so that case is reported as an error instead
    if let WakeData::QuasiSteady(_) = rust_simulation.wake_data {
        set_last_error(
            format!(
                "{}: the induced velocities are only available for simulations with a dynamic \
                 wake",
                function_name
            )
        );

        return -5;
    }

    let rust_points = unsafe { vectors_from_c(points, nr_points) };

    let induced_velocities = rust_simulation.induced_velocities(&rust_points);

    write_vectors(&induced_velocities, velocity_out, nr_points)
}

/// Helper that borrows the rust simulation, and reports a null pointer as an error.
fn simulation_ref<'a>(
    simulation: *mut LiftingLineSimulation,
    function_name: &str,
) -> Option<&'a SimulationImpl> {
    if simulation.is_null() {
        set_last_error(format!("{}: the simulation pointer is null", function_name));

        return None;
    }

    Some(unsafe { &*(simulation as *const SimulationImpl) })
}

/// Helper for the functions that take a single vector as input, which borrows the rust simulation
/// and converts the vector, and reports a null pointer as an error.
fn mutable_simulation_and_vector<'a>(
    simulation: *mut LiftingLineSimulation,
    vector: *const f64,
    function_name: &str,
) -> Option<(&'a mut SimulationImpl, SpatialVector)> {
    if simulation.is_null() || vector.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return None;
    }

    Some(
        (
            unsafe { &mut *(simulation as *mut SimulationImpl) },
            unsafe { vector_from_c(vector) }
        )
    )
}

/// Helper for the setters that need one value for each sail, which checks the input and returns
/// it as a slice, or the error code to report.
fn input_values_for_each_sail<'a>(
    simulation: *mut LiftingLineSimulation,
    values: *const f64,
    length: usize,
    value_name: &str,
    function_name: &str,
) -> Result<&'a [f64], i32> {
    if simulation.is_null() || values.is_null() {
        set_last_error(format!("{}: one of the input pointers is null", function_name));

        return Err(-1);
    }

    let rust_simulation = unsafe { &*(simulation as *const SimulationImpl) };

    let nr_sails = rust_simulation.line_force_model.nr_wings();

    if length != nr_sails {
        set_last_error(
            format!(
                "{}: got {} {}, but the simulation has {} sails",
                function_name,
                length,
                value_name,
                nr_sails
            )
        );

        return Err(-2);
    }

    Ok(unsafe { std::slice::from_raw_parts(values, length) })
}

/// Helper that reads a single vector from a caller owned array of 3 doubles.
///
/// # Safety
/// - `vector` must be a non-null pointer to an array with at least 3 doubles.
unsafe fn vector_from_c(vector: *const f64) -> SpatialVector {
    let values = unsafe { std::slice::from_raw_parts(vector, 3) };

    SpatialVector::from([values[0], values[1], values[2]])
}

/// Helper that reads a list of vectors from a caller owned array with 3 doubles per vector.
///
/// # Safety
/// - `vectors` must be a non-null pointer to an array with at least `3 * nr_vectors` doubles.
unsafe fn vectors_from_c(vectors: *const f64, nr_vectors: usize) -> Vec<SpatialVector> {
    let values = unsafe { std::slice::from_raw_parts(vectors, 3 * nr_vectors) };

    values.chunks_exact(3).map(
        |v| SpatialVector::from([v[0], v[1], v[2]])
    ).collect()
}

/// Helper that copies a list of vectors into a caller owned array, with 3 doubles per vector.
fn write_vectors(vectors: &[SpatialVector], vectors_out: *mut f64, nr_vectors: usize) -> i32 {
    let actual_count = vectors.len().min(nr_vectors);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(vectors_out, 3 * actual_count)
    };

    for (i, vector) in vectors.iter().take(actual_count).enumerate() {
        output_slice[3 * i..3 * i + 3].copy_from_slice(&vector.0);
    }

    actual_count as i32
}

/// Helper that copies a list of floats into a caller owned array.
fn write_floats(values: &[f64], values_out: *mut f64, values_out_length: usize) -> i32 {
    let actual_count = values.len().min(values_out_length);

    let output_slice = unsafe {
        std::slice::from_raw_parts_mut(values_out, actual_count)
    };

    output_slice.copy_from_slice(&values[0..actual_count]);

    actual_count as i32
}
