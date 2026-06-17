use crate::base::Base;
use crate::communication::Message;
use crate::map::{bfs_next_step, CellType, Map, Position, ResourceType};
use crate::robot::{Robot, RobotType};
use parking_lot::Mutex;
use rand::Rng;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

/// Snapshot of simulation state cloned for lock-free rendering
pub struct RenderSnapshot {
    pub map: Map,
    pub robot_positions: Vec<(u32, Position, RobotType)>,
    pub energy: u32,
    pub crystals: u32,
    pub turn: u32,
}

pub struct Simulation {
    pub robots: Vec<Arc<Mutex<Robot>>>,
    pub base: Arc<Base>,
    /// Knowledge base shared between all robots (updated via messages)
    pub shared_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    pub map: Arc<Mutex<Map>>,
    pub running: Arc<AtomicBool>,
    turn: u32,
    rx: mpsc::Receiver<Message>,
}

impl Simulation {
    pub fn new(width: usize, height: usize, num_scouts: u32, num_collectors: u32) -> Self {
        let map_data = Map::generate(width, height);
        let base_position = map_data.base_position;
        let map = Arc::new(Mutex::new(map_data));
        let base = Arc::new(Base::new());
        let shared_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<Message>();
        let running = Arc::new(AtomicBool::new(true));

        let mut robots: Vec<Arc<Mutex<Robot>>> = Vec::new();

        for i in 0..num_scouts {
            let robot = Arc::new(Mutex::new(Robot::new_scout(i, base_position, base_position)));
            robots.push(robot.clone());

            let map_c = map.clone();
            let tx_c = tx.clone();
            let running_c = running.clone();
            let robot_c = robot.clone();

            thread::spawn(move || run_scout(robot_c, map_c, tx_c, running_c));
        }

        for i in 0..num_collectors {
            let robot = Arc::new(Mutex::new(Robot::new_collector(
                num_scouts + i,
                base_position,
                base_position,
            )));
            robots.push(robot.clone());

            let map_c = map.clone();
            let tx_c = tx.clone();
            let running_c = running.clone();
            let robot_c = robot.clone();
            let shared_res_c = shared_resources.clone();
            let base_c = base.clone();

            thread::spawn(move || {
                run_collector(robot_c, map_c, tx_c, running_c, shared_res_c, base_c)
            });
        }

        Simulation {
            robots,
            base,
            shared_resources,
            map,
            running,
            turn: 0,
            rx,
        }
    }

    /// Process messages from robot threads and update shared knowledge base
    pub fn tick(&mut self) {
        self.turn += 1;
        while let Ok(msg) = self.rx.try_recv() {
            match &msg {
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
                Message::ResourceCollected {
                    position, quantity, ..
                } => {
                    let mut res = self.shared_resources.lock();
                    // Use a flag to avoid holding the mutable borrow while calling remove
                    let should_remove = if let Some(entry) = res.get_mut(position) {
                        if entry.1 <= *quantity {
                            true
                        } else {
                            entry.1 -= quantity;
                            false
                        }
                    } else {
                        false
                    };
                    if should_remove {
                        res.remove(position);
                    }
                }
                // Deposits are applied directly by the collector thread
                Message::ResourceDepositedAtBase { .. } => {}
                // Obstacle info is embedded in map, shared_resources is enough
                Message::ObstacleDiscovered { .. } => {}
            }
        }
    }

    /// Clone the current state for rendering without holding any locks during draw
    pub fn get_snapshot(&self) -> RenderSnapshot {
        let map = self.map.lock().clone();
        let robot_positions = self
            .robots
            .iter()
            .map(|r| {
                let r = r.lock();
                (r.id, r.position, r.robot_type)
            })
            .collect();
        RenderSnapshot {
            map,
            robot_positions,
            energy: self.base.get_total_energy(),
            crystals: self.base.get_total_crystals(),
            turn: self.turn,
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Scout thread: random walk, broadcast resource and obstacle discoveries
// ---------------------------------------------------------------------------
fn run_scout(
    robot: Arc<Mutex<Robot>>,
    map: Arc<Mutex<Map>>,
    tx: mpsc::Sender<Message>,
    running: Arc<AtomicBool>,
) {
    let mut rng = rand::thread_rng();

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));

        let (pos, id) = {
            let r = robot.lock();
            (r.position, r.id)
        };

        let (walkable, obstacles, resource_at): (Vec<Position>, Vec<Position>, Option<(Position, ResourceType, u32)>) = {
            let m = map.lock();
            let all = pos.neighbors_cardinal(m.width, m.height);
            let walkable: Vec<_> = all.iter().copied().filter(|&p| m.get_cell(p) != CellType::Obstacle).collect();
            let obstacles: Vec<_> = all.iter().copied().filter(|&p| m.get_cell(p) == CellType::Obstacle).collect();

            // Also scan current position for a resource (for first arrival)
            let resource_at = m.resources.get(&pos)
                .filter(|r| r.quantity > 0)
                .map(|r| (pos, r.resource_type, r.quantity));
            (walkable, obstacles, resource_at)
        };

        // Broadcast obstacles seen around current position
        for obs_pos in obstacles {
            let _ = tx.send(Message::ObstacleDiscovered { robot_id: id, position: obs_pos });
        }

        // Broadcast resource at current position (if any)
        if let Some((rpos, rt, qty)) = resource_at {
            let _ = tx.send(Message::ResourceDiscovered {
                robot_id: id,
                position: rpos,
                resource_type: rt,
                quantity: qty,
            });
        }

        // Move to a random walkable neighbor
        if !walkable.is_empty() {
            let next_pos = walkable[rng.gen_range(0..walkable.len())];

            // Discover resource at next position before moving
            let next_resource = {
                let m = map.lock();
                match m.get_cell(next_pos) {
                    CellType::Energy | CellType::Crystal => m
                        .resources
                        .get(&next_pos)
                        .filter(|r| r.quantity > 0)
                        .map(|r| (r.resource_type, r.quantity)),
                    _ => None,
                }
            };

            if let Some((rt, qty)) = next_resource {
                let _ = tx.send(Message::ResourceDiscovered {
                    robot_id: id,
                    position: next_pos,
                    resource_type: rt,
                    quantity: qty,
                });
            }

            robot.lock().position = next_pos;
        }
    }
}

// ---------------------------------------------------------------------------
// Collector thread: navigate to known resources, collect, return to base
// ---------------------------------------------------------------------------
fn run_collector(
    robot: Arc<Mutex<Robot>>,
    map: Arc<Mutex<Map>>,
    tx: mpsc::Sender<Message>,
    running: Arc<AtomicBool>,
    shared_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    base: Arc<Base>,
) {
    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));

        let (pos, id, inventory, carrying_type, target, base_pos) = {
            let r = robot.lock();
            (r.position, r.id, r.inventory, r.carrying_type, r.target, r.base_position)
        };

        if inventory > 0 {
            // --- Phase: return to base ---
            if pos == base_pos {
                let ct = carrying_type.unwrap_or(ResourceType::Energy);
                base.deposit_resource(ct, inventory);
                let _ = tx.send(Message::ResourceDepositedAtBase {
                    robot_id: id,
                    resource_type: ct,
                    quantity: inventory,
                });
                let mut r = robot.lock();
                r.inventory = 0;
                r.carrying_type = None;
                r.returning_to_base = false;
                r.target = None;
            } else {
                let next = { let m = map.lock(); bfs_next_step(&m, pos, base_pos) };
                if let Some(next_pos) = next {
                    robot.lock().position = next_pos;
                }
            }
        } else {
            // --- Phase: find and collect a resource ---
            let tgt = target.or_else(|| {
                // Pick the nearest known resource by Manhattan distance
                shared_resources
                    .lock()
                    .keys()
                    .min_by_key(|p| {
                        (p.x as i32 - pos.x as i32).abs() + (p.y as i32 - pos.y as i32).abs()
                    })
                    .copied()
            });

            match tgt {
                None => {
                    // No known resources yet — idle
                    robot.lock().target = None;
                }
                Some(tgt_pos) => {
                    if pos == tgt_pos {
                        // Try to collect one unit from the map
                        let (rt_opt, should_remove) = {
                            let mut m = map.lock();
                            let result = if let Some(res) = m.resources.get_mut(&tgt_pos) {
                                if res.quantity > 0 {
                                    let rt = res.resource_type;
                                    res.quantity -= 1;
                                    let depleted = res.quantity == 0;
                                    (Some(rt), depleted)
                                } else {
                                    (None, true)
                                }
                            } else {
                                (None, false)
                            };
                            result
                        };

                        // Clean up map if resource exhausted (separate lock to avoid borrow issue)
                        if should_remove {
                            let mut m = map.lock();
                            m.resources.remove(&tgt_pos);
                            m.cells[tgt_pos.y][tgt_pos.x] = CellType::Empty;
                            // Remove from shared knowledge too
                            shared_resources.lock().remove(&tgt_pos);
                        }

                        if let Some(rt) = rt_opt {
                            let _ = tx.send(Message::ResourceCollected {
                                robot_id: id,
                                position: tgt_pos,
                                resource_type: rt,
                                quantity: 1,
                            });
                            let mut r = robot.lock();
                            r.inventory = 1;
                            r.carrying_type = Some(rt);
                            r.returning_to_base = true;
                            r.target = Some(tgt_pos);
                        } else {
                            // Resource was already gone
                            shared_resources.lock().remove(&tgt_pos);
                            robot.lock().target = None;
                        }
                    } else {
                        // Move toward target using BFS (obstacle-aware)
                        let next = { let m = map.lock(); bfs_next_step(&m, pos, tgt_pos) };
                        if let Some(next_pos) = next {
                            let mut r = robot.lock();
                            r.position = next_pos;
                            r.target = Some(tgt_pos);
                        } else {
                            // Target unreachable — abandon it
                            shared_resources.lock().remove(&tgt_pos);
                            robot.lock().target = None;
                        }
                    }
                }
            }
        }
    }
}
