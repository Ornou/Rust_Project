use crate::map::{Position, ResourceType};
use parking_lot::Mutex;
use std::collections::{HashSet, HashMap};
use std::sync::Arc;
use rand::Rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RobotType {
    Scout,
    Collector,
}

pub struct Robot {
    pub id: u32,
    pub robot_type: RobotType,
    pub position: Position,
    pub inventory: u32,
    pub known_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    pub known_obstacles: Arc<Mutex<HashSet<Position>>>,
    pub base_position: Position,
    pub target: Option<Position>,
    pub returning_to_base: bool,
}

impl Robot {
    pub fn new_scout(
        id: u32,
        position: Position,
        base_position: Position,
        known_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
        known_obstacles: Arc<Mutex<HashSet<Position>>>,
    ) -> Self {
        Robot {
            id,
            robot_type: RobotType::Scout,
            position,
            inventory: 0,
            known_resources,
            known_obstacles,
            base_position,
            target: None,
            returning_to_base: false,
        }
    }

    pub fn new_collector(
        id: u32,
        position: Position,
        base_position: Position,
        known_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
        known_obstacles: Arc<Mutex<HashSet<Position>>>,
    ) -> Self {
        Robot {
            id,
            robot_type: RobotType::Collector,
            position,
            inventory: 0,
            known_resources,
            known_obstacles,
            base_position,
            target: None,
            returning_to_base: false,
        }
    }

    pub fn decide_next_move(&mut self, map_width: usize, map_height: usize) -> Option<Position> {
        let mut rng = rand::thread_rng();

        match self.robot_type {
            RobotType::Scout => {
                // Explore randomly
                let mut best_pos = None;
                let neighbors = self.position.neighbors(map_width, map_height);

                for neighbor in neighbors {
                    if !self.known_obstacles.lock().contains(&neighbor) {
                        if best_pos.is_none() || rng.gen_bool(0.5) {
                            best_pos = Some(neighbor);
                        }
                    }
                }

                best_pos
            }
            RobotType::Collector => {
                if self.inventory > 0 && self.returning_to_base {
                    // Return to base
                    self.move_towards(self.base_position)
                } else if self.inventory == 0 && self.target.is_none() {
                    // Pick a target resource
                    let resources = self.known_resources.lock();
                    if !resources.is_empty() {
                        let positions: Vec<_> = resources.keys().copied().collect();
                        if !positions.is_empty() {
                            self.target = Some(positions[rng.gen_range(0..positions.len())]);
                        }
                    }
                    drop(resources);
                    self.move_towards(self.target.unwrap_or(self.base_position))
                } else if let Some(target) = self.target {
                    self.move_towards(target)
                } else {
                    self.move_towards(self.base_position)
                }
            }
        }
    }

    fn move_towards(&self, target: Position) -> Option<Position> {
        let dx = if target.x > self.position.x {
            1
        } else if target.x < self.position.x {
            -1
        } else {
            0
        };

        let dy = if target.y > self.position.y {
            1
        } else if target.y < self.position.y {
            -1
        } else {
            0
        };

        if dx == 0 && dy == 0 {
            return None;
        }

        let new_x = ((self.position.x as i32) + dx) as usize;
        let new_y = ((self.position.y as i32) + dy) as usize;

        Some(Position::new(new_x, new_y))
    }

    pub fn add_resource_knowledge(&self, pos: Position, resource_type: ResourceType, qty: u32) {
        self.known_resources.lock().insert(pos, (resource_type, qty));
    }

    pub fn add_obstacle_knowledge(&self, pos: Position) {
        self.known_obstacles.lock().insert(pos);
    }
}
