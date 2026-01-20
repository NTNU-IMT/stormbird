pub mod lifting_line;

#[unsafe(no_mangle)]
pub extern "C" fn test_function() -> i32 {
    42
}

// Re-export all public C functions so cbindgen can find them
pub use lifting_line::complete_sail_model::{
    CCompleteSailModel,
    complete_sail_model_new,
    complete_sail_model_drop,
    complete_sail_model_do_step,
    complete_sail_model_simulate_condition,
};

/// Error codes that can be returned from functions
#[repr(C)]
#[derive(Debug)]
pub enum CStormbirdResult {
    Success = 0,
    CouldNotConvertToRustString = 1,
    InvalidInitialization = 2,
    InputIsNull = 3,
    Unknown = 99,
}
