use color_eyre::{eyre::WrapErr, Result};
use context_attribute::context;
use framework::AdditionalOutput;
use hardware::ActuatorInterface;
use serde::{Deserialize, Serialize};
use types::{
    collected_commands::CollectedCommands, joints::Joints, motion_selection::MotionSafeExits,
};

#[derive(Deserialize, Serialize)]
pub struct JointCommandSender {}

#[context]
pub struct CreationContext {}

#[context]
pub struct CycleContext {
    final_positions: AdditionalOutput<Joints<f32>, "final_positions">,
    stiffnesses: AdditionalOutput<Joints<f32>, "stiffnesses">,
    motion_safe_exits_output: AdditionalOutput<MotionSafeExits, "motion_safe_exits_output">,

    motion_safe_exits: CyclerState<MotionSafeExits, "motion_safe_exits">,
    collected_commands: Input<CollectedCommands, "collected_commands">,

    hardware_interface: HardwareInterface,
}

#[context]
#[derive(Default)]
pub struct MainOutputs {}

impl JointCommandSender {
    pub fn new(_context: CreationContext) -> Result<Self> {
        Ok(Self {})
    }

    pub fn cycle(
        &mut self,
        mut context: CycleContext<impl ActuatorInterface>,
    ) -> Result<MainOutputs> {
        let collected_commands = context.collected_commands;

        // The actuators uses the raw sensor data (not corrected like current_positions) in their feedback loops,
        // thus the compensation is required to make them reach the actual desired position.
        context
            .hardware_interface
            .write_to_actuators(
                collected_commands.positions,
                collected_commands.stiffnesses,
                collected_commands.leds,
            )
            .wrap_err("failed to write to actuators")?;

        context
            .final_positions
            .fill_if_subscribed(|| collected_commands.positions);
        context
            .stiffnesses
            .fill_if_subscribed(|| collected_commands.stiffnesses);
        context
            .motion_safe_exits_output
            .fill_if_subscribed(|| context.motion_safe_exits.clone());

        Ok(MainOutputs {})
    }
}
