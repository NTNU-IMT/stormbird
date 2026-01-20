

use stormbird::lifting_line::complete_sail_model::CompleteSailModel;


use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use crate::CStormbirdResult;


#[repr(C)]
pub struct CCompleteSailModel {
    _private: [u8; 0],
}

#[repr(C)]
pub struct CSimulationResult {
    _private: [u8; 0],
}


#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_new(
    setup_string: *const c_char,
    out_model: *mut *mut CCompleteSailModel
) -> CStormbirdResult {
    if setup_string.is_null() || out_model.is_null() {
        return CStormbirdResult::InputIsNull;
    }
    
    let setup_str: String = match unsafe { CStr::from_ptr(setup_string) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return CStormbirdResult::CouldNotConvertToRustString,
    };
    
    let sail_model_res = CompleteSailModel::new_from_string(&setup_str);
    
    match sail_model_res {
        Ok(model) => {
            let boxed_model = Box::new(model);
            let model_ptr = Box::into_raw(boxed_model);
            
            unsafe {
                *out_model = model_ptr as *mut CCompleteSailModel;
            }
            
            CStormbirdResult::Success
        },
        Err(_) => {
            unsafe {
                *out_model = std::ptr::null_mut();
            }
            
            CStormbirdResult::InvalidInitialization
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_drop(sail_model: *mut CCompleteSailModel) {
    if !sail_model.is_null() {
        unsafe {
            let _ = Box::from_raw(sail_model as *mut CompleteSailModel);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_do_step(
    sail_model: *mut CCompleteSailModel,
    time: f64,
    time_step: f64, 
    wind_velocity: f64,
    wind_direction: f64,
    ship_velocity: f64,
    controller_loading: f64
) -> CStormbirdResult {
    
    if sail_model.is_null() {
        return CStormbirdResult::InputIsNull;
    }
        
    // Convert back to Rust type
    let rust_model = unsafe { &mut *(sail_model as *mut CompleteSailModel) };
    
    todo!();
}

#[unsafe(no_mangle)]
pub extern "C" fn complete_sail_model_simulate_condition(
    sail_model: &mut CCompleteSailModel,
    wind_velocity: f64,
    wind_direction: f64,
    ship_velocity: f64,
    controller_loading: f64,
    time_step: f64,
    nr_time_steps: usize
) -> CStormbirdResult {
    todo!();
}
