use stormbird::actuator_line::ActuatorLine;
use stormath::type_aliases::Float;

pub struct ActuatorLineInterface {
    pub model: ActuatorLine,
    pub dominating_line_indices: Vec<usize>,
    pub summed_projection_weights: Vec<Float>
}
