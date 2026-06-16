use crate::map::{Map, Position, ResourceType};
use crate::robot::{Robot, RobotType};
use crate::base::Base;
use crate::communication::{Message, MessageBroker};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct Simulation {
    pub map: Map,
    pub robots: Vec<Arc<Mutex<Robot>>>,
    pub base: Arc<Base>,
    pub message_broker: Arc<Mutex<MessageBroker>>,
    pub shared_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    pub shared_obstacles: Arc<Mutex<HashSet<Position>>>,
    pub turn: u32,
}

impl Simulation {
    pub fn new(width: usize, height: usize, num_scouts: u32, num_collectors: u32) -> Self {
        let map = Map::generate(width, height);
        let base = Arc::new(Base::new());
        let shared_resources = Arc::new(Mutex::new(HashMap::new()));
        let shared_obstacles = Arc::new(Mutex::new(HashSet::new()));

        let mut robots = Vec::new();

        // Create scouts
        for i in 0..num_scouts {
            let robot = Robot::new_scout(
                i,
                map.base_position,
                map.base_position,
                shared_resources.clone(),
                shared_obstacles.clone(),
            );
            robots.push(Arc::new(Mutex::new(robot)));
        }

        // Create collectors
        for i in 0..num_collectors {
            let robot = Robot::new_collector(
                num_scouts + i,
                map.base_position,
                map.base_position,
                shared_resources.clone(),
                shared_obstacles.clone(),
            );
            robots.push(Arc::new(Mutex::new(robot)));
        }

        Simulation {
            map,
            robots,
            base,
            message_broker: Arc::new(Mutex::new(MessageBroker::new())),
            shared_resources,
            shared_obstacles,
            turn: 0,
        }
    }

    pub fn tick(&mut self) {
        self.turn += 1;

        // Move robots
        for robot_arc in self.robots.iter() {
            let mut robot = robot_arc.lock();

            if let Some(next_pos) = robot.decide_next_move(self.map.width, self.map.height) {
                if self.map.is_walkable(next_pos) {
                    robot.position = next_pos;

                    // Scout discovers resources and obstacles
                    if robot.robot_type == RobotType::Scout {
                        let cell = self.map.get_cell(next_pos);
                        match cell {
                            crate::map::CellType::Energy => {
                                if let Some(resource) = self.map.resources.get(&next_pos) {
                                    let mut broker = self.message_broker.lock();
                                    broker.broadcast(Message::ResourceDiscovered {
                                        robot_id: robot.id,
                                        position: next_pos,
                                        resource_type: ResourceType::Energy,
                                        quantity: resource.quantity,
                                    });
                                    robot.add_resource_knowledge(
                                        next_pos,
                                        ResourceType::Energy,
                                        resource.quantity,
                                    );
                                }
                            }
                            crate::map::CellType::Crystal => {
                                if let Some(resource) = self.map.resources.get(&next_pos) {
                                    let mut broker = self.message_broker.lock();
                                    broker.broadcast(Message::ResourceDiscovered {
                                        robot_id: robot.id,
                                        position: next_pos,
                                        resource_type: ResourceType::Crystal,
                                        quantity: resource.quantity,
                                    });
                                    robot.add_resource_knowledge(
                                        next_pos,
                                        ResourceType::Crystal,
                                        resource.quantity,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    // Collector collects and returns
                    else if robot.robot_type == RobotType::Collector {
                        if next_pos == robot.base_position && robot.inventory > 0 {
                            // Determine resource type from known resources
                            let known = robot.known_resources.lock();
                            let mut resource_type = ResourceType::Energy;
                            if let Some((rt, _)) = known.get(&next_pos) {
                                resource_type = *rt;
                            }
                            drop(known);

                            let mut broker = self.message_broker.lock();
                            broker.broadcast(Message::ResourceDepositedAtBase {
                                robot_id: robot.id,
                                resource_type,
                                quantity: robot.inventory,
                            });
                            self.base.deposit_resource(resource_type, robot.inventory);
                            robot.inventory = 0;
                            robot.returning_to_base = false;
                            robot.target = None;
                        } else if self.map.resources.contains_key(&next_pos) && robot.inventory == 0
                        {
                            // Collect one unit
                            if let Some(resource) = self.map.resources.get_mut(&next_pos) {
                                if resource.quantity > 0 {
                                    robot.inventory += 1;
                                    resource.quantity -= 1;

                                    let mut broker = self.message_broker.lock();
                                    broker.broadcast(Message::ResourceCollected {
                                        robot_id: robot.id,
                                        position: next_pos,
                                        resource_type: resource.resource_type,
                                        quantity: 1,
                                    });
                                    robot.returning_to_base = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Process messages
        let mut broker = self.message_broker.lock();
        for message in &broker.messages {
            match message {
                Message::ResourceDiscovered {
                    position,
                    resource_type,
                    quantity,
                    ..
                } => {
                    self.shared_resources
                        .lock()
                        .insert(*position, (*resource_type, *quantity));
                }
                _ => {}
            }
        }
        broker.clear();
    }

    pub fn get_robot_positions(&self) -> Vec<(u32, Position, RobotType)> {
        self.robots
            .iter()
            .map(|r| {
                let robot = r.lock();
                (robot.id, robot.position, robot.robot_type)
            })
            .collect()
    }
}
