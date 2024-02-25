use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use coordinate_systems::{Isometry2, Point2, Vector2};
use serialize_hierarchy::SerializeHierarchy;
use spl_network_messages::PlayerNumber;

use crate::{
    coordinate_systems::{Field, Ground},
    fall_state::FallState,
    filtered_game_controller_state::FilteredGameControllerState,
    kick_decision::KickDecision,
    obstacles::Obstacle,
    penalty_shot_direction::PenaltyShotDirection,
    primary_state::PrimaryState,
    roles::Role,
    rule_obstacles::RuleObstacle,
    support_foot::Side,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct WorldState {
    pub ball: Option<BallState>,
    pub rule_ball: Option<BallState>,
    pub filtered_game_controller_state: Option<FilteredGameControllerState>,
    pub obstacles: Vec<Obstacle>,
    pub rule_obstacles: Vec<RuleObstacle>,
    pub position_of_interest: Point2<Ground>,
    pub kick_decisions: Option<Vec<KickDecision>>,
    pub instant_kick_decisions: Option<Vec<KickDecision>>,
    pub robot: RobotState,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, SerializeHierarchy)]
pub struct BallState {
    pub ball_in_ground: Point2<Ground>,
    pub ball_in_field: Point2<Field>,
    pub ball_in_ground_velocity: Vector2<Ground>,
    pub last_seen_ball: SystemTime,
    pub penalty_shot_direction: Option<PenaltyShotDirection>,
    pub field_side: Side,
}

impl BallState {
    pub fn new_at_center(ground_to_field: Isometry2<Ground, Field>) -> Self {
        Self {
            ball_in_field: Point2::origin(),
            ball_in_ground: ground_to_field.inverse() * Point2::origin(),
            ball_in_ground_velocity: Vector2::zeros(),
            last_seen_ball: UNIX_EPOCH,
            penalty_shot_direction: Default::default(),
            field_side: Side::Left,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, SerializeHierarchy)]
pub struct RobotState {
    pub ground_to_field: Option<Isometry2<Ground, Field>>,
    pub role: Role,
    pub primary_state: PrimaryState,
    pub fall_state: FallState,
    pub has_ground_contact: bool,
    pub player_number: PlayerNumber,
}
