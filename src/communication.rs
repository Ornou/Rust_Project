use crate::map::{Position, ResourceType};

#[derive(Debug, Clone)]
pub enum Message {
    ResourceDiscovered {
        robot_id: u32,
        position: Position,
        resource_type: ResourceType,
        quantity: u32,
    },
    ObstacleDiscovered {
        robot_id: u32,
        position: Position,
    },
    ResourceCollected {
        robot_id: u32,
        position: Position,
        resource_type: ResourceType,
        quantity: u32,
    },
    ResourceDepositedAtBase {
        robot_id: u32,
        resource_type: ResourceType,
        quantity: u32,
    },
}

impl Message {
    pub fn robot_id(&self) -> u32 {
        match self {
            Message::ResourceDiscovered { robot_id, .. } => *robot_id,
            Message::ObstacleDiscovered { robot_id, .. } => *robot_id,
            Message::ResourceCollected { robot_id, .. } => *robot_id,
            Message::ResourceDepositedAtBase { robot_id, .. } => *robot_id,
        }
    }
}
