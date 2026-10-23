// Copyright (C) 2024, NTNU
// Author: Jarle Vinje Kramer <jarlekramer@gmail.com; jarle.a.kramer@ntnu.no>
// License: GPL v3.0 (see separate file LICENSE or https://www.gnu.org/licenses/gpl-3.0.html)

//! Model of the fans driving the internal flow in a foil with active air flow, based on fan
//! curves.

use serde::{Serialize, Deserialize};

use stormath::type_aliases::Float;
use stormath::consts::MIN_POSITIVE;
use stormath::interpolation::linear_interpolation;
use stormath::solvers::{brent_solver, SolverSettings};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "FanCurveDataInput")]
/// Data points for the fan curves. The meaning of the values depends on the variant of
/// [`FanCurves`] the data is used in.
///
/// The data must only cover the stable operating range of the fan, where the pressure rise
/// decreases with increasing flow. This is checked when the data is deserialized.
pub struct FanCurveData {
    /// Volume flow, or flow coefficient, in increasing order
    pub flow_data: Vec<Float>,
    /// Pressure rise, or pressure coefficient, as a function of the flow
    pub pressure_data: Vec<Float>,
    /// Input power, or power coefficient, as a function of the flow
    pub power_data: Vec<Float>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// Unchecked version of the fan curve data, used for deserialization
struct FanCurveDataInput {
    flow_data: Vec<Float>,
    pressure_data: Vec<Float>,
    power_data: Vec<Float>,
}

impl TryFrom<FanCurveDataInput> for FanCurveData {
    type Error = String;

    fn try_from(input: FanCurveDataInput) -> Result<Self, Self::Error> {
        Self::new(input.flow_data, input.pressure_data, input.power_data)
    }
}

impl FanCurveData {
    /// Creates new fan curve data, and checks that the data is valid
    pub fn new(
        flow_data: Vec<Float>,
        pressure_data: Vec<Float>,
        power_data: Vec<Float>
    ) -> Result<Self, String> {
        let n = flow_data.len();

        if n < 2 {
            return Err("The fan curves must have at least two data points".to_string());
        }

        if pressure_data.len() != n || power_data.len() != n {
            return Err(format!(
                "The fan curve data must have the same length. Flow: {}, pressure: {}, power: {}",
                n, pressure_data.len(), power_data.len()
            ));
        }

        if flow_data[0] < 0.0 {
            return Err("The flow data for the fan curves can not be negative".to_string());
        }

        for i in 1..n {
            if flow_data[i] <= flow_data[i - 1] {
                return Err("The flow data for the fan curves must be strictly increasing".to_string());
            }

            if pressure_data[i] >= pressure_data[i - 1] {
                return Err(
                    "The pressure data for the fan curves must be strictly decreasing with \
                    increasing flow. Only the stable operating range of the fan should be included"
                    .to_string()
                );
            }
        }

        if power_data.iter().any(|&power| power < 0.0) {
            return Err("The power data for the fan curves can not be negative".to_string());
        }

        Ok(Self { flow_data, pressure_data, power_data })
    }

    /// Interpolates the pressure data. Below the first data point, the first value is used. Above
    /// the last data point, the curve is linearly extrapolated, so that the pressure continues to
    /// decrease with increasing flow.
    pub fn pressure(&self, flow: Float) -> Float {
        Self::interpolate_with_extrapolation(flow, &self.flow_data, &self.pressure_data)
    }

    /// Interpolates the power data, in the same way as for the pressure, but limited to positive
    /// values.
    pub fn power(&self, flow: Float) -> Float {
        Self::interpolate_with_extrapolation(flow, &self.flow_data, &self.power_data).max(0.0)
    }

    fn interpolate_with_extrapolation(x: Float, x_data: &[Float], y_data: &[Float]) -> Float {
        let n = x_data.len();

        if x > x_data[n - 1] {
            let slope = (y_data[n - 1] - y_data[n - 2]) / (x_data[n - 1] - x_data[n - 2]);

            y_data[n - 1] + slope * (x - x_data[n - 1])
        } else {
            linear_interpolation(x, x_data, y_data)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Fan curves for a single fan, giving the pressure rise and power as a function of the volume
/// flow. The curves are scaled to different rotational speeds, n, and densities, rho, using the fan
/// affinity laws. The rotational speed is given in revolutions per second.
pub enum FanCurves {
    /// Dimensional curves, measured at a reference rotational speed and density. The data is
    /// given as volume flow in m^3/s, pressure rise in Pa and power in W. The scaling to other
    /// speeds and densities is:
    /// Q = Q_ref * (n / n_ref), dp = dp_ref * (rho / rho_ref) * (n / n_ref)^2,
    /// P = P_ref * (rho / rho_ref) * (n / n_ref)^3
    Dimensional {
        /// The rotational speed of the measured curves, in revolutions per second
        reference_rotational_speed: Float,
        /// The density of the measured curves
        reference_density: Float,
        /// The dimensional fan curve data
        data: FanCurveData,
    },
    /// Non-dimensional curves, given as the flow coefficient, Q / (n * D^3), the pressure
    /// coefficient, dp / (rho * n^2 * D^2), and the power coefficient, P / (rho * n^3 * D^5),
    /// where D is the fan diameter.
    NonDimensional {
        /// The diameter of the fan
        diameter: Float,
        /// The non-dimensional fan curve data
        data: FanCurveData,
    },
}

impl FanCurves {
    fn data(&self) -> &FanCurveData {
        match self {
            Self::Dimensional { data, .. } => data,
            Self::NonDimensional { data, .. } => data,
        }
    }

    /// Returns the scales for the flow, pressure and power, at a given rotational speed and
    /// density, so that the value of a quantity is the value in the data times the scale.
    fn scales(&self, rotational_speed: Float, density: Float) -> (Float, Float, Float) {
        match self {
            Self::Dimensional { reference_rotational_speed, reference_density, .. } => {
                let speed_ratio = rotational_speed / reference_rotational_speed;
                let density_ratio = density / reference_density;

                (
                    speed_ratio,
                    density_ratio * speed_ratio.powi(2),
                    density_ratio * speed_ratio.powi(3)
                )
            },
            Self::NonDimensional { diameter, .. } => {
                (
                    rotational_speed * diameter.powi(3),
                    density * rotational_speed.powi(2) * diameter.powi(2),
                    density * rotational_speed.powi(3) * diameter.powi(5)
                )
            },
        }
    }

    /// The pressure rise over the fan for a given volume flow, rotational speed and density. A
    /// fan that is not rotating gives no pressure rise.
    pub fn pressure_rise(&self, volume_flow: Float, rotational_speed: Float, density: Float) -> Float {
        if rotational_speed < MIN_POSITIVE {
            return 0.0;
        }

        let (flow_scale, pressure_scale, _) = self.scales(rotational_speed, density);

        self.data().pressure(volume_flow / flow_scale) * pressure_scale
    }

    /// The input power to the fan for a given volume flow, rotational speed and density. A fan
    /// that is not rotating needs no power.
    pub fn power(&self, volume_flow: Float, rotational_speed: Float, density: Float) -> Float {
        if rotational_speed < MIN_POSITIVE {
            return 0.0;
        }

        let (flow_scale, _, power_scale) = self.scales(rotational_speed, density);

        self.data().power(volume_flow / flow_scale) * power_scale
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Model of the fan system that drives the internal flow in a wing. The system can consist of
/// multiple identical fans in parallel, which share the total volume flow equally.
pub struct Fan {
    /// The fan curves for a single fan
    pub curves: FanCurves,
    #[serde(default = "Fan::default_number_of_fans")]
    /// The number of identical fans in parallel
    pub number_of_fans: usize,
    #[serde(default)]
    /// Coefficient for the pressure loss in the ducts between the internal volume of the wing and
    /// the fans, as a function of the total volume flow: dp_duct = k * Q^2. Given in Pa s^2 / m^6.
    pub duct_loss_coefficient: Float,
}

impl Fan {
    /// Default number of fans
    pub fn default_number_of_fans() -> usize {1}

    fn number_of_fans_float(&self) -> Float {
        self.number_of_fans.max(1) as Float
    }

    /// The pressure rise over the fans for a given total volume flow
    pub fn pressure_rise(&self, total_volume_flow: Float, rotational_speed: Float, density: Float) -> Float {
        self.curves.pressure_rise(
            total_volume_flow / self.number_of_fans_float(), rotational_speed, density
        )
    }

    /// The total input power to the fans for a given total volume flow
    pub fn power(&self, total_volume_flow: Float, rotational_speed: Float, density: Float) -> Float {
        self.number_of_fans_float() * self.curves.power(
            total_volume_flow / self.number_of_fans_float(), rotational_speed, density
        )
    }

    /// The pressure loss in the ducts between the internal volume and the fans
    pub fn duct_loss(&self, total_volume_flow: Float) -> Float {
        self.duct_loss_coefficient * total_volume_flow.powi(2)
    }

    /// The largest pressure rise the fans can deliver at a given rotational speed, which is at the
    /// lowest flow in the fan curves.
    pub fn max_pressure_rise(&self, rotational_speed: Float, density: Float) -> Float {
        self.pressure_rise(0.0, rotational_speed, density)
    }

    /// Solves for the rotational speed that gives the required pressure rise at a given total
    /// volume flow. Returns zero if the required pressure rise is zero or negative.
    pub fn rotational_speed_for_operating_point(
        &self,
        total_volume_flow: Float,
        required_pressure_rise: Float,
        density: Float,
        settings: &SolverSettings,
    ) -> Float {
        if required_pressure_rise < MIN_POSITIVE {
            return 0.0;
        }

        // The pressure rise at a fixed flow increases with the rotational speed, as long as the
        // pressure decreases with increasing flow in the fan curves.
        let residual = |rotational_speed: Float| {
            self.pressure_rise(total_volume_flow, rotational_speed, density) /
                required_pressure_rise - 1.0
        };

        let mut upper_speed = 1.0;
        let mut nr_expansions = 0;

        while residual(upper_speed) < 0.0 && nr_expansions < 64 {
            upper_speed *= 2.0;
            nr_expansions += 1;
        }

        brent_solver(residual, 0.0, upper_speed, settings).unwrap_or(upper_speed)
    }
}
