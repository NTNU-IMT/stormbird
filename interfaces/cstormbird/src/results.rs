use stormbird::common_utils::results::simplfied::SingleSailResult as SingleSailResultImpl;

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
