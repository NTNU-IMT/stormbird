// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Interface to the wind conditions used as input to the simulation models.

use std::ffi::CStr;
use std::os::raw::c_char;

use crate::error::set_last_error;

use stormbird::wind::wind_condition::WindCondition as WindConditionImpl;
use stormbird::wind::wind_condition::discretized_spectrum::DiscretizedSpectrum;
use stormbird::wind::wind_condition::velocity_variation::VelocityVariation;
use stormbird::wind::wind_condition::velocity_variation::power_model::PowerModel;
use stormbird::wind::wind_condition::velocity_variation::logarithmic_model::LogarithmicModel;

/// Opaque pointer structure to a WindCondition.
///
/// On the rust side the velocity variation with height is an enum with a different structure in
/// each variant, and the optional gust spectrums contain variable sized data. Neither can be
/// represented directly over a C-ABI, so the wind condition is handled as an opaque handle that is
/// created by one of the `wind_condition_new_*` functions. A handle must always be released with
/// `wind_condition_drop`.
#[repr(C)]
pub struct WindCondition {
    _private: [u8; 0],
}

/// Wraps a wind condition from the rust library in a heap allocation owned by the caller.
fn box_wind_condition(wind_condition: WindConditionImpl) -> *mut WindCondition {
    Box::into_raw(Box::new(wind_condition)) as *mut WindCondition
}

/// Borrows the rust wind condition behind a handle. The caller must check for null first.
///
/// # Safety
/// - `wind_condition` must be a non-null pointer returned by one of the `wind_condition_new_*`
///   functions.
pub(crate) unsafe fn wind_condition_ref<'a>(
    wind_condition: *const WindCondition
) -> &'a WindConditionImpl {
    unsafe { &*(wind_condition as *const WindConditionImpl) }
}

/// Creates a wind condition with the same velocity at all heights.
///
/// `direction_coming_from` is the direction the wind is coming from, in radians.
///
/// # Safety
/// - The returned pointer must be freed with `wind_condition_drop`
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_new_constant(
    direction_coming_from: f64,
    velocity: f64,
) -> *mut WindCondition {
    box_wind_condition(
        WindConditionImpl {
            direction_coming_from,
            velocity_variation: VelocityVariation::Constant(velocity),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    )
}

/// Creates a wind condition where the velocity varies with height according to the power law
/// model for the atmospheric boundary layer.
///
/// `direction_coming_from` is the direction the wind is coming from, in radians.
/// `reference_velocity` is the velocity at `reference_height`. `power_factor` controls the shape
/// of the profile. Use `wind_condition_new_power_model_with_default_shape` to get the standard
/// values for the two latter.
///
/// # Safety
/// - The returned pointer must be freed with `wind_condition_drop`
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_new_power_model(
    direction_coming_from: f64,
    reference_velocity: f64,
    reference_height: f64,
    power_factor: f64,
) -> *mut WindCondition {
    box_wind_condition(
        WindConditionImpl {
            direction_coming_from,
            velocity_variation: VelocityVariation::PowerModel(
                PowerModel {
                    reference_velocity,
                    reference_height,
                    power_factor
                }
            ),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    )
}

/// Same as `wind_condition_new_power_model`, but with the default reference height of 10 m and
/// the default power factor of 1/9 recommended by the ITTC.
///
/// # Safety
/// - The returned pointer must be freed with `wind_condition_drop`
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_new_power_model_with_default_shape(
    direction_coming_from: f64,
    reference_velocity: f64,
) -> *mut WindCondition {
    wind_condition_new_power_model(
        direction_coming_from,
        reference_velocity,
        PowerModel::default_reference_height(),
        PowerModel::default_power_factor()
    )
}

/// Creates a wind condition where the velocity varies with height according to a logarithmic
/// model, which may include a correction for a non-neutral atmosphere.
///
/// `direction_coming_from` is the direction the wind is coming from, in radians. An
/// `obukhov_length` of exactly 0.0 means that no stability correction is applied. The Von Karman
/// constant and the stability coefficients are set to their default values, and can be changed
/// afterwards with the corresponding setters.
///
/// # Safety
/// - The returned pointer must be freed with `wind_condition_drop`
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_new_logarithmic_model(
    direction_coming_from: f64,
    friction_velocity: f64,
    surface_roughness: f64,
    obukhov_length: f64,
) -> *mut WindCondition {
    let obukhov_length_rust = if obukhov_length != 0.0 {
        Some(obukhov_length)
    } else {
        None
    };

    box_wind_condition(
        WindConditionImpl {
            direction_coming_from,
            velocity_variation: VelocityVariation::LogarithmicModel(
                LogarithmicModel {
                    friction_velocity,
                    surface_roughness,
                    obukhov_length: obukhov_length_rust,
                    von_karman_constant: LogarithmicModel::default_von_karman_constant(),
                    stable_coefficient: LogarithmicModel::default_stable_coefficient(),
                    unstable_coefficient: LogarithmicModel::default_unstable_coefficient()
                }
            ),
            parallel_gust: None,
            perpendicular_gust: None,
            vertical_gust: None
        }
    )
}

/// Creates a wind condition from a JSON string, using the same format as the setup files. This is
/// the only way to set every variable of the wind condition, including the gust spectrums, in a
/// single call.
///
/// Returns NULL if the pointer is null, or if the string is not valid UTF-8 or valid JSON for a
/// wind condition.
///
/// # Safety
/// - `json_string` must be a valid null-terminated C string
/// - The returned pointer must be freed with `wind_condition_drop`
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_new_from_json_string(
    json_string: *const c_char
) -> *mut WindCondition {
    let json_str = match string_from_c(json_string) {
        Some(s) => s,
        None => {
            set_last_error(
                "wind_condition_new_from_json_string: the input string is null or not valid UTF-8"
            );

            return std::ptr::null_mut();
        }
    };

    match serde_json::from_str::<WindConditionImpl>(json_str) {
        Ok(wind_condition) => box_wind_condition(wind_condition),
        Err(error) => {
            set_last_error(
                format!("wind_condition_new_from_json_string: {}", error)
            );

            std::ptr::null_mut()
        }
    }
}

/// Frees a wind condition returned by one of the `wind_condition_new_*` functions.
///
/// # Safety
/// - `wind_condition` must be a pointer returned by this library, and must not be used afterwards.
/// - Calling this function with a null pointer is allowed, and does nothing.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_drop(wind_condition: *mut WindCondition) {
    if !wind_condition.is_null() {
        unsafe {
            let _ = Box::from_raw(wind_condition as *mut WindConditionImpl);
        }
    }
}

/// The direction the wind is coming from, in radians. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_direction_coming_from(
    wind_condition: *const WindCondition
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error("wind_condition_get_direction_coming_from");
    }

    unsafe { wind_condition_ref(wind_condition) }.direction_coming_from
}

/// Sets the direction the wind is coming from, in radians.
///
/// Returns 0 on success, or -1 if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_direction_coming_from(
    wind_condition: *mut WindCondition,
    direction_coming_from: f64,
) -> i32 {
    if wind_condition.is_null() {
        set_last_error("wind_condition_set_direction_coming_from: the wind condition pointer is null");

        return -1;
    }

    let rust_condition = unsafe { &mut *(wind_condition as *mut WindConditionImpl) };

    rust_condition.direction_coming_from = direction_coming_from;

    0
}

/// Sets the gust spectrum for the velocity component parallel to the true wind, from the discrete
/// harmonic components of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_components` is zero.
///
/// # Safety
/// - `frequencies`, `amplitudes` and `phases` must all point to arrays with at least
///   `nr_components` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_parallel_gust(
    wind_condition: *mut WindCondition,
    frequencies: *const f64,
    amplitudes: *const f64,
    phases: *const f64,
    nr_components: usize,
) -> i32 {
    set_gust(
        wind_condition,
        frequencies,
        amplitudes,
        phases,
        nr_components,
        GustComponent::Parallel
    )
}

/// Sets the gust spectrum for the velocity component perpendicular to the true wind, from the
/// discrete harmonic components of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_components` is zero.
///
/// # Safety
/// - `frequencies`, `amplitudes` and `phases` must all point to arrays with at least
///   `nr_components` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_perpendicular_gust(
    wind_condition: *mut WindCondition,
    frequencies: *const f64,
    amplitudes: *const f64,
    phases: *const f64,
    nr_components: usize,
) -> i32 {
    set_gust(
        wind_condition,
        frequencies,
        amplitudes,
        phases,
        nr_components,
        GustComponent::Perpendicular
    )
}

/// Sets the gust spectrum for the vertical velocity component, from the discrete harmonic
/// components of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -2 if
/// `nr_components` is zero.
///
/// # Safety
/// - `frequencies`, `amplitudes` and `phases` must all point to arrays with at least
///   `nr_components` doubles.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_vertical_gust(
    wind_condition: *mut WindCondition,
    frequencies: *const f64,
    amplitudes: *const f64,
    phases: *const f64,
    nr_components: usize,
) -> i32 {
    set_gust(
        wind_condition,
        frequencies,
        amplitudes,
        phases,
        nr_components,
        GustComponent::Vertical
    )
}

/// Sets the gust spectrum for the velocity component parallel to the true wind, from a JSON
/// string with the frequencies, amplitudes and phases of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -3 if the string is
/// not valid UTF-8 or valid JSON for a gust spectrum.
///
/// # Safety
/// - `gust_string` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_parallel_gust_from_json_string(
    wind_condition: *mut WindCondition,
    gust_string: *const c_char,
) -> i32 {
    set_gust_from_json_string(wind_condition, gust_string, GustComponent::Parallel)
}

/// Sets the gust spectrum for the velocity component perpendicular to the true wind, from a JSON
/// string with the frequencies, amplitudes and phases of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -3 if the string is
/// not valid UTF-8 or valid JSON for a gust spectrum.
///
/// # Safety
/// - `gust_string` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_perpendicular_gust_from_json_string(
    wind_condition: *mut WindCondition,
    gust_string: *const c_char,
) -> i32 {
    set_gust_from_json_string(wind_condition, gust_string, GustComponent::Perpendicular)
}

/// Sets the gust spectrum for the vertical velocity component, from a JSON string with the
/// frequencies, amplitudes and phases of the spectrum.
///
/// Returns 0 on success, or a negative error code: -1 if a pointer is null, -3 if the string is
/// not valid UTF-8 or valid JSON for a gust spectrum.
///
/// # Safety
/// - `gust_string` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_vertical_gust_from_json_string(
    wind_condition: *mut WindCondition,
    gust_string: *const c_char,
) -> i32 {
    set_gust_from_json_string(wind_condition, gust_string, GustComponent::Vertical)
}

/// The steady true wind velocity at the given height. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_steady_true_wind_velocity_at_height(
    wind_condition: *const WindCondition,
    height: f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error("wind_condition_steady_true_wind_velocity_at_height");
    }

    unsafe { wind_condition_ref(wind_condition) }.steady_true_wind_velocity_at_height(height)
}

/// The true wind velocity component parallel to the steady wind, including the gust spectrum, at
/// the given height and time. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_unsteady_parallel_true_wind_velocity_at_height(
    wind_condition: *const WindCondition,
    height: f64,
    time: f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error(
            "wind_condition_unsteady_parallel_true_wind_velocity_at_height"
        );
    }

    unsafe { wind_condition_ref(wind_condition) }
        .unsteady_parallel_true_wind_velocity_at_height(height, time)
}

/// The true wind velocity component perpendicular to the steady wind at the given time. This is
/// zero unless a perpendicular gust spectrum is set. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_unsteady_perpendicular_true_wind_velocity(
    wind_condition: *const WindCondition,
    time: f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error("wind_condition_unsteady_perpendicular_true_wind_velocity");
    }

    unsafe { wind_condition_ref(wind_condition) }.unsteady_perpendicular_true_wind_velocity(time)
}

/// The vertical true wind velocity component at the given time. This is zero unless a vertical
/// gust spectrum is set. Returns NaN if the input pointer is null.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_unsteady_vertical_true_wind_velocity(
    wind_condition: *const WindCondition,
    time: f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error("wind_condition_unsteady_vertical_true_wind_velocity");
    }

    unsafe { wind_condition_ref(wind_condition) }.unsteady_vertical_true_wind_velocity(time)
}

/// The unscaled Businger-Dyer correction for a non-neutral atmosphere at the given height.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_businger_dyer_unscaled_correction(
    wind_condition: *const WindCondition,
    height: f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error("wind_condition_businger_dyer_unscaled_correction");
    }

    match unsafe { wind_condition_ref(wind_condition) }.velocity_variation {
        VelocityVariation::LogarithmicModel(model) => {
            model.businger_dyer_unscaled_correction(height)
        },
        _ => 0.0
    }
}

/// The friction velocity of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_friction_velocity(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_friction_velocity",
        |model| model.friction_velocity
    )
}

/// Sets the friction velocity of the logarithmic model.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_friction_velocity(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_friction_velocity",
        |model| model.friction_velocity = value
    )
}

/// The surface roughness of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_surface_roughness(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_surface_roughness",
        |model| model.surface_roughness
    )
}

/// Sets the surface roughness of the logarithmic model.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_surface_roughness(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_surface_roughness",
        |model| model.surface_roughness = value
    )
}

/// The Obukhov length of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero both if the velocity variation is not a
/// logarithmic model and if no Obukhov length is set.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_obukhov_length(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_obukhov_length",
        |model| if let Some(length) = model.obukhov_length { length } else { 0.0 }
    )
}

/// Sets the Obukhov length of the logarithmic model. A value of exactly 0.0 removes the stability
/// correction.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_obukhov_length(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    let obukhov_length = if value != 0.0 { Some(value) } else { None };

    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_obukhov_length",
        |model| model.obukhov_length = obukhov_length
    )
}

/// The Von Karman constant of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_von_karman_constant(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_von_karman_constant",
        |model| model.von_karman_constant
    )
}

/// Sets the Von Karman constant of the logarithmic model.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_von_karman_constant(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_von_karman_constant",
        |model| model.von_karman_constant = value
    )
}

/// The coefficient used by the stable stability correction of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_stable_coefficient(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_stable_coefficient",
        |model| model.stable_coefficient
    )
}

/// Sets the coefficient used by the stable stability correction of the logarithmic model.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_stable_coefficient(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_stable_coefficient",
        |model| model.stable_coefficient = value
    )
}

/// The coefficient used by the unstable stability correction of the logarithmic model.
///
/// Returns NaN if the input pointer is null, and zero if the velocity variation is not a
/// logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_get_unstable_coefficient(
    wind_condition: *const WindCondition
) -> f64 {
    logarithmic_model_value(
        wind_condition,
        "wind_condition_get_unstable_coefficient",
        |model| model.unstable_coefficient
    )
}

/// Sets the coefficient used by the unstable stability correction of the logarithmic model.
///
/// Returns 0 on success, -1 if the input pointer is null, and -4 if the velocity variation is not
/// a logarithmic model.
#[unsafe(no_mangle)]
pub extern "C" fn wind_condition_set_unstable_coefficient(
    wind_condition: *mut WindCondition,
    value: f64,
) -> i32 {
    set_logarithmic_model_value(
        wind_condition,
        "wind_condition_set_unstable_coefficient",
        |model| model.unstable_coefficient = value
    )
}

/// The three velocity components a gust spectrum can be set for.
#[derive(Clone, Copy)]
enum GustComponent {
    Parallel,
    Perpendicular,
    Vertical
}

impl GustComponent {
    /// The name of the function that sets this component from arrays, used in error messages.
    fn setter_name(&self) -> &'static str {
        match self {
            Self::Parallel => "wind_condition_set_parallel_gust",
            Self::Perpendicular => "wind_condition_set_perpendicular_gust",
            Self::Vertical => "wind_condition_set_vertical_gust"
        }
    }

    /// The name of the function that sets this component from JSON, used in error messages.
    fn json_setter_name(&self) -> &'static str {
        match self {
            Self::Parallel => "wind_condition_set_parallel_gust_from_json_string",
            Self::Perpendicular => "wind_condition_set_perpendicular_gust_from_json_string",
            Self::Vertical => "wind_condition_set_vertical_gust_from_json_string"
        }
    }
}

/// Helper that stores a gust spectrum in the component it belongs to.
fn store_gust(
    wind_condition: &mut WindConditionImpl,
    spectrum: DiscretizedSpectrum,
    component: GustComponent,
) {
    match component {
        GustComponent::Parallel => wind_condition.parallel_gust = Some(spectrum),
        GustComponent::Perpendicular => wind_condition.perpendicular_gust = Some(spectrum),
        GustComponent::Vertical => wind_condition.vertical_gust = Some(spectrum)
    }
}

/// Helper that builds a gust spectrum from caller owned arrays.
fn set_gust(
    wind_condition: *mut WindCondition,
    frequencies: *const f64,
    amplitudes: *const f64,
    phases: *const f64,
    nr_components: usize,
    component: GustComponent,
) -> i32 {
    if wind_condition.is_null()
        || frequencies.is_null()
        || amplitudes.is_null()
        || phases.is_null()
    {
        set_last_error(
            format!("{}: one of the input pointers is null", component.setter_name())
        );

        return -1;
    }

    if nr_components == 0 {
        set_last_error(
            format!("{}: the gust spectrum has no components", component.setter_name())
        );

        return -2;
    }

    let spectrum = unsafe {
        DiscretizedSpectrum {
            frequencies: std::slice::from_raw_parts(frequencies, nr_components).to_vec(),
            amplitudes: std::slice::from_raw_parts(amplitudes, nr_components).to_vec(),
            phases: std::slice::from_raw_parts(phases, nr_components).to_vec()
        }
    };

    let rust_condition = unsafe { &mut *(wind_condition as *mut WindConditionImpl) };

    store_gust(rust_condition, spectrum, component);

    0
}

/// Helper that builds a gust spectrum from a JSON string.
fn set_gust_from_json_string(
    wind_condition: *mut WindCondition,
    gust_string: *const c_char,
    component: GustComponent,
) -> i32 {
    if wind_condition.is_null() {
        set_last_error(
            format!("{}: the wind condition pointer is null", component.json_setter_name())
        );

        return -1;
    }

    let gust_str = match string_from_c(gust_string) {
        Some(s) => s,
        None => {
            set_last_error(
                format!(
                    "{}: the gust string is null or not valid UTF-8",
                    component.json_setter_name()
                )
            );

            return -3;
        }
    };

    let spectrum = match serde_json::from_str::<DiscretizedSpectrum>(gust_str) {
        Ok(spectrum) => spectrum,
        Err(error) => {
            set_last_error(format!("{}: {}", component.json_setter_name(), error));

            return -3;
        }
    };

    let rust_condition = unsafe { &mut *(wind_condition as *mut WindConditionImpl) };

    store_gust(rust_condition, spectrum, component);

    0
}

/// Helper for the getters that are only defined for the logarithmic model.
fn logarithmic_model_value(
    wind_condition: *const WindCondition,
    function_name: &str,
    get_value: impl Fn(&LogarithmicModel) -> f64,
) -> f64 {
    if wind_condition.is_null() {
        return null_condition_error(function_name);
    }

    match unsafe { wind_condition_ref(wind_condition) }.velocity_variation {
        VelocityVariation::LogarithmicModel(ref model) => get_value(model),
        _ => {
            set_last_error(
                format!(
                    "{}: the wind condition does not use a logarithmic velocity variation",
                    function_name
                )
            );

            0.0
        }
    }
}

/// Helper for the setters that are only defined for the logarithmic model.
fn set_logarithmic_model_value(
    wind_condition: *mut WindCondition,
    function_name: &str,
    set_value: impl FnOnce(&mut LogarithmicModel),
) -> i32 {
    if wind_condition.is_null() {
        set_last_error(format!("{}: the wind condition pointer is null", function_name));

        return -1;
    }

    let rust_condition = unsafe { &mut *(wind_condition as *mut WindConditionImpl) };

    match rust_condition.velocity_variation {
        VelocityVariation::LogarithmicModel(ref mut model) => {
            set_value(model);
            0
        },
        _ => {
            set_last_error(
                format!(
                    "{}: the wind condition does not use a logarithmic velocity variation",
                    function_name
                )
            );

            -4
        }
    }
}

/// Helper that reports a null wind condition pointer for the functions that return a value.
fn null_condition_error(function_name: &str) -> f64 {
    set_last_error(format!("{}: the wind condition pointer is null", function_name));

    f64::NAN
}

/// Helper that converts a C string to a rust string slice, if it is valid UTF-8.
fn string_from_c<'a>(c_string: *const c_char) -> Option<&'a str> {
    if c_string.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(c_string) }.to_str().ok()
}
