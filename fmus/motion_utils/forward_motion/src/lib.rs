pub use fmu_from_struct::prelude::*;

#[derive(Fmu, Debug, Clone, Default)]
#[fmi_version = 2]
pub struct ForwardMotion {
    #[parameter]
    pub velocity: f64,
    #[output]
    pub x_position: f64,
}

impl FmuFunctions for ForwardMotion {
    fn do_step(&mut self, _current_time: f64, time_step: f64) {
        self.x_position += self.velocity * time_step;
    }
}
