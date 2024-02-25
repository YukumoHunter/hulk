use color_eyre::Result;
use nalgebra::{Rotation3, UnitQuaternion};
use serde::{Deserialize, Serialize};

use context_attribute::context;
use coordinate_systems::{point, IntoTransform, Isometry3};
use framework::{AdditionalOutput, MainOutput};
use projection::Projection;
use types::{
    camera_matrix::{CameraMatrices, CameraMatrix, ProjectedFieldLines},
    camera_position::CameraPosition,
    coordinate_systems::{Camera, Ground, Head, Pixel, Robot},
    field_dimensions::FieldDimensions,
    line::{Line, Line2},
    parameters::CameraMatrixParameters,
    robot_dimensions::RobotDimensions,
    robot_kinematics::RobotKinematics,
};

#[derive(Deserialize, Serialize)]
pub struct CameraMatrixCalculator {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    projected_field_lines: AdditionalOutput<ProjectedFieldLines, "projected_field_lines">,

    robot_kinematics: Input<RobotKinematics, "robot_kinematics">,
    robot_to_ground: RequiredInput<Option<Isometry3<Robot, Ground>>, "robot_to_ground?">,

    bottom_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_bottom">,
    field_dimensions: Parameter<FieldDimensions, "field_dimensions">,
    top_camera_matrix_parameters:
        Parameter<CameraMatrixParameters, "camera_matrix_parameters.vision_top">,

    correction_in_camera_top: CyclerState<Rotation3<f32>, "correction_in_camera_top">,
    correction_in_camera_bottom: CyclerState<Rotation3<f32>, "correction_in_camera_bottom">,
    correction_in_robot: CyclerState<Rotation3<f32>, "correction_in_robot">,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {
    pub camera_matrices: MainOutput<Option<CameraMatrices>>,
}

impl CameraMatrixCalculator {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(&mut self, mut context: CycleContext) -> Result<MainOutputs> {
        let image_size = point![640.0, 480.0];
        let top_camera_to_head = camera_to_head(
            CameraPosition::Top,
            context.top_camera_matrix_parameters.extrinsic_rotations,
        );
        let top_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.top_camera_matrix_parameters.focal_lengths,
            context.top_camera_matrix_parameters.cc_optical_center,
            image_size,
            top_camera_to_head,
            context.robot_kinematics.head_to_robot,
            *context.robot_to_ground,
        );

        let bottom_camera_to_head = camera_to_head(
            CameraPosition::Bottom,
            context.bottom_camera_matrix_parameters.extrinsic_rotations,
        );
        let bottom_camera_matrix = CameraMatrix::from_normalized_focal_and_center(
            context.bottom_camera_matrix_parameters.focal_lengths,
            context.bottom_camera_matrix_parameters.cc_optical_center,
            image_size,
            bottom_camera_to_head,
            context.robot_kinematics.head_to_robot,
            *context.robot_to_ground,
        );

        let field_dimensions = context.field_dimensions;
        context
            .projected_field_lines
            .fill_if_subscribed(|| ProjectedFieldLines {
                top: project_penalty_area_on_images(field_dimensions, &top_camera_matrix)
                    .unwrap_or_default(),
                bottom: project_penalty_area_on_images(field_dimensions, &bottom_camera_matrix)
                    .unwrap_or_default(),
            });
        Ok(MainOutputs {
            camera_matrices: Some(CameraMatrices {
                top: top_camera_matrix.to_corrected(
                    *context.correction_in_robot,
                    *context.correction_in_camera_top,
                ),
                bottom: bottom_camera_matrix.to_corrected(
                    *context.correction_in_robot,
                    *context.correction_in_camera_bottom,
                ),
            })
            .into(),
        })
    }
}

pub fn camera_to_head(
    camera_position: CameraPosition,
    extrinsic_rotation: nalgebra::Vector3<f32>,
) -> Isometry3<Camera, Head> {
    let extrinsic_angles_in_radians = extrinsic_rotation.map(|a: f32| a.to_radians());
    let extrinsic_rotation = UnitQuaternion::from_euler_angles(
        extrinsic_angles_in_radians.x,
        extrinsic_angles_in_radians.y,
        extrinsic_angles_in_radians.z,
    );
    let neck_to_camera = match camera_position {
        CameraPosition::Top => RobotDimensions::NECK_TO_TOP_CAMERA,
        CameraPosition::Bottom => RobotDimensions::NECK_TO_BOTTOM_CAMERA,
    };
    let camera_pitch = match camera_position {
        CameraPosition::Top => 1.2f32.to_radians(),
        CameraPosition::Bottom => 39.7f32.to_radians(),
    };
    (nalgebra::Isometry3::from(neck_to_camera)
        * nalgebra::Isometry3::rotation(nalgebra::Vector3::y() * camera_pitch)
        * extrinsic_rotation)
        .framed_transform()
}

fn project_penalty_area_on_images(
    field_dimensions: &FieldDimensions,
    camera_matrix: &CameraMatrix,
) -> Option<Vec<Line2<Pixel>>> {
    let field_length = &field_dimensions.length;
    let field_width = &field_dimensions.width;
    let penalty_area_length = &field_dimensions.penalty_area_length;
    let penalty_area_width = &field_dimensions.penalty_area_width;

    let penalty_top_left = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, penalty_area_width / 2.0])
        .ok()?;
    let penalty_top_right = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, -penalty_area_width / 2.0])
        .ok()?;
    let penalty_bottom_left = camera_matrix
        .ground_to_pixel(point![
            field_length / 2.0 - penalty_area_length,
            penalty_area_width / 2.0
        ])
        .ok()?;
    let penalty_bottom_right = camera_matrix
        .ground_to_pixel(point![
            field_length / 2.0 - penalty_area_length,
            -penalty_area_width / 2.0
        ])
        .ok()?;
    let corner_left = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, field_width / 2.0])
        .ok()?;
    let corner_right = camera_matrix
        .ground_to_pixel(point![field_length / 2.0, -field_width / 2.0])
        .ok()?;

    Some(vec![
        Line(penalty_top_left, penalty_top_right),
        Line(penalty_bottom_left, penalty_bottom_right),
        Line(penalty_bottom_left, penalty_top_left),
        Line(penalty_bottom_right, penalty_top_right),
        Line(corner_left, corner_right),
    ])
}
