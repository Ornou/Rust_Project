use crate::map::{Position, ResourceType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RobotType {
    Scout,
    Collector,
}

pub struct Robot {
    pub id: u32,
    pub robot_type: RobotType,
    pub position: Position,
    /// Units currently being carried back to base
    pub inventory: u32,
    /// Type of resource in the inventory
    pub carrying_type: Option<ResourceType>,
    pub base_position: Position,
    /// Current collection target (Collector only)
    pub target: Option<Position>,
    pub returning_to_base: bool,
}

impl Robot {
    pub fn new_scout(id: u32, position: Position, base_position: Position) -> Self {
        Robot {
            id,
            robot_type: RobotType::Scout,
            position,
            inventory: 0,
            carrying_type: None,
            base_position,
            target: None,
            returning_to_base: false,
        }
    }

    pub fn new_collector(id: u32, position: Position, base_position: Position) -> Self {
        Robot {
            id,
            robot_type: RobotType::Collector,
            position,
            inventory: 0,
            carrying_type: None,
            base_position,
            target: None,
            returning_to_base: false,
        }
    }
}
