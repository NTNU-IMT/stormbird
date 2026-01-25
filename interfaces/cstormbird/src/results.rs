pub const MAX_NUMBER_OF_SAILS: usize = 20;

use stormbird::common_utils::results::simplfied::SingleSailResult as SingleSailResultImpl;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SingleSailResult {
    force: [f64; 3],
    moment: [f64; 3],
    input_power: f64
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

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SailResults {
    pub data: [SingleSailResult; MAX_NUMBER_OF_SAILS],
    pub length: usize
}

impl SailResults {
    pub fn from_rust_array(input: Vec<SingleSailResultImpl>) -> Self {
        let length = input.len().min(MAX_NUMBER_OF_SAILS);
        
        let mut out = SailResults::default();
        out.length = length;
        
        for i in 0..length {
            out.data[i] = SingleSailResult::from(input[i].clone())
        }
        
        out
    }
}