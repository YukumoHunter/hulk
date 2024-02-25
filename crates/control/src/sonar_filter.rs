use color_eyre::Result;
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::point;
use filtering::low_pass_filter::LowPassFilter;
use framework::{AdditionalOutput, MainOutput};
use types::{
    fall_state::FallState, sensor_data::SensorData, sonar_obstacle::SonarObstacle,
    sonar_values::SonarValues,
};

#[derive(Deserialize, Serialize)]
pub struct SonarFilter {
    filtered_sonar_left: LowPassFilter<f32>,
    filtered_sonar_right: LowPassFilter<f32>,
}

#[context]
pub struct CreationContext {
    low_pass_filter_coefficient: Parameter<f32, "sonar_filter.low_pass_filter_coefficient">,
    maximal_detectable_distance: Parameter<f32, "sonar_filter.maximal_detectable_distance">,
}

#[context]
pub struct CycleContext {
    sonar_values: AdditionalOutput<SonarValues, "sonar_values">,

    maximal_reliable_distance: Parameter<f32, "sonar_filter.maximal_reliable_distance">,
    minimal_reliable_distance: Parameter<f32, "sonar_filter.minimal_reliable_distance">,
    middle_merge_threshold: Parameter<f32, "sonar_filter.middle_merge_threshold">,
    sensor_angle: Parameter<f32, "sonar_obstacle.sensor_angle">,

    fall_state: Input<FallState, "fall_state">,
    sensor_data: Input<SensorData, "sensor_data">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub sonar_obstacles: MainOutput<Vec<SonarObstacle>>,
}

impl SonarFilter {
    pub fn new(context: CreationContext) -> Result<Self> {
        Ok(Self {
            filtered_sonar_left: LowPassFilter::with_smoothing_factor(
                *context.maximal_detectable_distance,
                *context.low_pass_filter_coefficient,
            ),
            filtered_sonar_right: LowPassFilter::with_smoothing_factor(
                *context.maximal_detectable_distance,
                *context.low_pass_filter_coefficient,
            ),
        })
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let sonar_sensors = &context.sensor_data.sonar_sensors;
        let fall_state = context.fall_state;

        self.filtered_sonar_left.update(sonar_sensors.left);
        self.filtered_sonar_right.update(sonar_sensors.right);

        let acceptance_range =
            *context.minimal_reliable_distance..*context.maximal_reliable_distance;

        let obstacle_detected_on_left =
            (acceptance_range).contains(&self.filtered_sonar_left.state());
        let obstacle_detected_on_right =
            (acceptance_range).contains(&self.filtered_sonar_right.state());

        context.sonar_values.fill_if_subscribed(|| SonarValues {
            left_sonar: obstacle_detected_on_left,
            right_sonar: obstacle_detected_on_right,
            filtered_left_sonar_value: self.filtered_sonar_left.state(),
            filtered_right_sonar_value: self.filtered_sonar_right.state(),
        });

        let left_point = point![
            context.sensor_angle.cos() * self.filtered_sonar_left.state(),
            context.sensor_angle.sin() * self.filtered_sonar_left.state()
        ];
        let right_point = point![
            context.sensor_angle.cos() * self.filtered_sonar_right.state(),
            -context.sensor_angle.sin() * self.filtered_sonar_right.state()
        ];
        let middle_point = point![
            (self.filtered_sonar_left.state() + self.filtered_sonar_right.state()) / 2.0,
            0.0
        ];

        let obstacle_positions = match (
            fall_state,
            obstacle_detected_on_left,
            obstacle_detected_on_right,
        ) {
            (FallState::Upright, true, true) => {
                if (self.filtered_sonar_left.state() - self.filtered_sonar_right.state()).abs()
                    < *context.middle_merge_threshold
                {
                    vec![middle_point]
                } else {
                    vec![left_point, right_point]
                }
            }
            (FallState::Upright, true, false) => vec![left_point],
            (FallState::Upright, false, true) => vec![right_point],
            _ => vec![],
        };
        let sonar_obstacles: Vec<_> = obstacle_positions
            .into_iter()
            .map(|position| SonarObstacle { position })
            .collect();

        Ok(MainOutputs {
            sonar_obstacles: sonar_obstacles.into(),
        })
    }
}
