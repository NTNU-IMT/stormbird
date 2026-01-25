
use std::ffi::CStr;
use std::os::raw::c_char;

use crate::wind::WindCondition;
use crate::results::{
    SingleSailResult,
    SailResults,
    MAX_NUMBER_OF_SAILS
};

use stormbird::{
    lifting_line::complete_sail_model::CompleteSailModel as CompleteSailModelImpl,
    wind::wind_condition::WindCondition as WindConditionImpl
};


/// Opaque pointer structure to the CompleteSailModel
#[repr(C)]
pub struct CompleteSailModel {
    _private: [u8; 0],
}

/// Creates a new CompleteSailModel from a JSON setup string.
/// Returns NULL on error.
///
/// # Safety
/// - `setup_string` must be a valid null-terminated C string
/// - The returned pointer must be freed with `complete_sail_model_drop`
#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_new(setup_string: *const c_char) -> *mut CompleteSailModel {
    if setup_string.is_null() {
        return std::ptr::null_mut();
    }
    
    let setup_str = match unsafe { CStr::from_ptr(setup_string) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return std::ptr::null_mut(),
    };
    
    match CompleteSailModelImpl::new_from_string(&setup_str) {
        Ok(model) => Box::into_raw(Box::new(model)) as *mut CompleteSailModel,
        Err(_) => std::ptr::null_mut()
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

#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_steady_state_condition(
    sail_model: *mut CompleteSailModel,
    wind_condition: WindCondition,
    ship_velocity: f64,
    controller_loading: f64
) -> SailResults {
    // Check for null pointer
    if sail_model.is_null() {
        return SailResults {
            data: [SingleSailResult::default(); MAX_NUMBER_OF_SAILS],
            length: 0,
        };
    }
    
    let rust_model = unsafe {
        &mut *(sail_model as *mut CompleteSailModelImpl)
    };
    
    let wind_condition_rust = WindConditionImpl::from(wind_condition);
    
    let rust_result = rust_model.simulate_steady_state_condition_simple_output(
        wind_condition_rust, 
        ship_velocity, 
        controller_loading
    );
    
    SailResults::from_rust_array(rust_result)
}
