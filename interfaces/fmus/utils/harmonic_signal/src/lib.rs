//! An FMU that produces a harmonic signal as output, based on amplitude, frequency and phase shift
//! as parameters.
//!
//! Intended to be used for simple experiments, for instance to test other FMUs with oscillatory
//! inputs.

use fmu_from_struct::prelude::*;

use std::f64::consts::PI;

#[derive(Fmu, Debug, Clone, Default)]
#[fmu_from_struct(fmi_version = 2)]
pub struct HarmonicSignal {
    #[fmu_from_struct(parameter)]
    pub amplitude: f64,
    pub period: f64,
    pub phase_shift_in_deg: f64,
    #[fmu_from_struct(output)]
    pub signal: f64,

    frequency: f64,
    phase_shift: f64,
}

impl FmuFunctions for HarmonicSignal {
    fn exit_initialization_mode(&mut self) {
        self.frequency = 2.0 * PI / self.period;
        self.phase_shift = self.phase_shift_in_deg.to_radians();
    }

    fn do_step(&mut self, current_time: f64, _time_step: f64) {
        self.signal = self.amplitude * (self.frequency * current_time + self.phase_shift).sin();
    }
}
