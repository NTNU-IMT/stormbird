pub mod lifting_line;
pub mod results;
pub mod wind;

// Re-export all public C functions so cbindgen can find them
pub use lifting_line::complete_sail_model::{
    CompleteSailModel,
    complete_sail_model_new,
    complete_sail_model_drop,
};

pub use wind::{
    WindCondition,
};
