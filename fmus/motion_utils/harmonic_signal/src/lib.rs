//! An FMU that produces a harmonic signal as output, based on amplitude, frequency and phase shift
//! as parameters.
//! 
//! Intended to be used for simple experiments, for instance to test other FMUs with oscillatory 
//! inputs.

pub use fmu_from_struct::prelude::*;

#[derive(Fmu, Debug, Clone, Default)]
#[fmi_version = 2]
pub struct HarmonicSignal {
    #[parameter]
    pub amplitude: f64,
    pub frequency: f64,
    pub phase_shift: f64,
    #[output]
    pub signal: f64,
}

impl FmuFunctions for HarmonicSignal {
    fn do_step(&mut self, current_time: f64, _time_step: f64) {
        self.signal = self.amplitude * (self.frequency * current_time + self.phase_shift).sin();
    }
}