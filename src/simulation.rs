use crate::base::Base;
use crate::communication::Message;
use crate::map::{bfs_next_step, CellType, Map, Position, ResourceType};
use crate::robot::{Robot, RobotType};
use parking_lot::Mutex;
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
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
    pub fn new(
        width: usize,
        height: usize,
        num_scouts: u32,
        num_collectors: u32,
        seed: Option<u64>,
    ) -> Self {
        let seed_value = seed.unwrap_or(0);
        let mut rng = ChaCha8Rng::seed_from_u64(seed_value);
        let map_data = Map::generate(width, height, &mut rng);
        let base_position = map_data.base_position;
        let map = Arc::new(Mutex::new(map_data));
        let base = Arc::new(Base::new());
        let shared_resources: Arc<Mutex<HashMap<Position, (ResourceType, u32)>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let (tx, rx) = mpsc::channel::<Message>();
        let running = Arc::new(AtomicBool::new(true));

        tracing::info!(
            seed = seed_value,
            width,
            height,
            num_scouts,
            num_collectors,
            "Simulation created"
        );

        let mut robots: Vec<Arc<Mutex<Robot>>> =
            Vec::with_capacity((num_scouts + num_collectors) as usize);

        for i in 0..num_scouts {
            let robot = Arc::new(Mutex::new(Robot::new_scout(
                i,
                base_position,
                base_position,
            )));
            robots.push(robot.clone());

            let map_c = map.clone();
            let tx_c = tx.clone();
            let running_c = running.clone();
            let robot_c = robot.clone();
            let thread_seed = rng.next_u64();

            thread::spawn(move || run_scout(robot_c, map_c, tx_c, running_c, thread_seed));
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
            let thread_seed = rng.next_u64();

            thread::spawn(move || {
                run_collector(
                    robot_c,
                    map_c,
                    tx_c,
                    running_c,
                    shared_res_c,
                    base_c,
                    thread_seed,
                )
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
            self.handle_message(msg);
        }
    }

    fn handle_message(&mut self, msg: Message) {
        let robot_id = msg.robot_id();
        match msg {
            Message::ResourceDiscovered {
                position,
                resource_type,
                quantity,
                ..
            } => {
                tracing::debug!(
                    robot_id,
                    ?position,
                    ?resource_type,
                    quantity,
                    "resource discovered"
                );
                self.handle_resource_discovered(position, resource_type, quantity)
            }
            Message::ResourceCollected {
                position,
                resource_type,
                quantity,
                ..
            } => {
                tracing::debug!(
                    robot_id,
                    ?position,
                    ?resource_type,
                    quantity,
                    "resource collected"
                );
                self.handle_resource_collected(position, quantity)
            }
            Message::ResourceDepositedAtBase {
                resource_type,
                quantity,
                ..
            } => {
                tracing::debug!(
                    robot_id,
                    ?resource_type,
                    quantity,
                    "resource deposited at base"
                );
            }
            Message::ObstacleDiscovered { position, .. } => {
                tracing::debug!(robot_id, ?position, "obstacle discovered");
            }
        }
    }

    fn handle_resource_discovered(
        &self,
        position: Position,
        resource_type: ResourceType,
        quantity: u32,
    ) {
        self.shared_resources
            .lock()
            .insert(position, (resource_type, quantity));
    }

    fn handle_resource_collected(&self, position: Position, quantity: u32) {
        let mut resources = self.shared_resources.lock();
        if let Some(entry) = resources.get_mut(&position) {
            if entry.1 <= quantity {
                resources.remove(&position);
            } else {
                entry.1 -= quantity;
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
    seed: u64,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));

        let (pos, id) = {
            let r = robot.lock();
            (r.position, r.id)
        };

        let (walkable, obstacles, resource_at) = {
            let m = map.lock();
            scan_surroundings(&m, pos)
        };

        for obs_pos in obstacles {
            let _ = tx.send(Message::ObstacleDiscovered {
                robot_id: id,
                position: obs_pos,
            });
        }

        if let Some((rpos, rt, qty)) = resource_at {
            let _ = tx.send(Message::ResourceDiscovered {
                robot_id: id,
                position: rpos,
                resource_type: rt,
                quantity: qty,
            });
        }

        if let Some(next_pos) = choose_random_neighbor(&mut rng, &walkable) {
            if let Some((rt, qty)) = scan_resource_at(&map, next_pos) {
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

fn scan_surroundings(
    map: &Map,
    pos: Position,
) -> (
    Vec<Position>,
    Vec<Position>,
    Option<(Position, ResourceType, u32)>,
) {
    let all = pos.neighbors_cardinal(map.width, map.height);
    let walkable = all
        .iter()
        .copied()
        .filter(|&p| map.get_cell(p) != CellType::Obstacle)
        .collect();
    let obstacles = all
        .iter()
        .copied()
        .filter(|&p| map.get_cell(p) == CellType::Obstacle)
        .collect();
    let resource_at = map
        .resources
        .get(&pos)
        .and_then(|r| (r.quantity > 0).then_some((pos, r.resource_type, r.quantity)));
    (walkable, obstacles, resource_at)
}

fn choose_random_neighbor(rng: &mut ChaCha8Rng, walkable: &[Position]) -> Option<Position> {
    if walkable.is_empty() {
        None
    } else {
        Some(walkable[rng.gen_range(0..walkable.len())])
    }
}

fn scan_resource_at(map: &Arc<Mutex<Map>>, pos: Position) -> Option<(ResourceType, u32)> {
    let m = map.lock();
    match m.get_cell(pos) {
        CellType::Energy | CellType::Crystal => m
            .resources
            .get(&pos)
            .filter(|r| r.quantity > 0)
            .map(|r| (r.resource_type, r.quantity)),
        _ => None,
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
    seed: u64,
) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    while running.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));

        let (pos, id, inventory, carrying_type, target, base_pos) = {
            let r = robot.lock();
            (
                r.position,
                r.id,
                r.inventory,
                r.carrying_type,
                r.target,
                r.base_position,
            )
        };

        if inventory > 0 {
            handle_return_to_base(
                &robot,
                &map,
                base.clone(),
                tx.clone(),
                id,
                carrying_type,
                inventory,
                pos,
                base_pos,
            );
            continue;
        }

        let tgt_pos = choose_target(pos, target, &shared_resources, &mut rng);
        if let Some(tgt_pos) = tgt_pos {
            handle_collector_target(&robot, &map, &shared_resources, &tx, id, pos, tgt_pos);
        } else {
            robot.lock().target = None;
        }
    }
}

fn choose_target(
    pos: Position,
    current_target: Option<Position>,
    shared_resources: &Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    rng: &mut ChaCha8Rng,
) -> Option<Position> {
    current_target.or_else(|| {
        shared_resources
            .lock()
            .iter()
            .min_by(|(a, _), (b, _)| {
                let a_dist = (a.x as i32 - pos.x as i32).abs() + (a.y as i32 - pos.y as i32).abs();
                let b_dist = (b.x as i32 - pos.x as i32).abs() + (b.y as i32 - pos.y as i32).abs();
                a_dist.cmp(&b_dist).then_with(|| {
                    if rng.gen_bool(0.5) {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
            })
            .map(|(p, _)| *p)
    })
}

fn handle_return_to_base(
    robot: &Arc<Mutex<Robot>>,
    map: &Arc<Mutex<Map>>,
    base: Arc<Base>,
    tx: mpsc::Sender<Message>,
    id: u32,
    carrying_type: Option<ResourceType>,
    inventory: u32,
    pos: Position,
    base_pos: Position,
) {
    if pos != base_pos {
        let next = {
            let m = map.lock();
            bfs_next_step(&m, pos, base_pos)
        };
        if let Some(next_pos) = next {
            robot.lock().position = next_pos;
        }
        return;
    }

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
}

fn handle_collector_target(
    robot: &Arc<Mutex<Robot>>,
    map: &Arc<Mutex<Map>>,
    shared_resources: &Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    tx: &mpsc::Sender<Message>,
    id: u32,
    pos: Position,
    tgt_pos: Position,
) {
    if pos == tgt_pos {
        match try_collect_resource(map, shared_resources, tgt_pos) {
            Some(rt) => {
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
            }
            None => {
                shared_resources.lock().remove(&tgt_pos);
                robot.lock().target = None;
            }
        }
    } else {
        let next = {
            let m = map.lock();
            bfs_next_step(&m, pos, tgt_pos)
        };
        if let Some(next_pos) = next {
            let mut r = robot.lock();
            r.position = next_pos;
            r.target = Some(tgt_pos);
        } else {
            shared_resources.lock().remove(&tgt_pos);
            robot.lock().target = None;
        }
    }
}

fn try_collect_resource(
    map: &Arc<Mutex<Map>>,
    shared_resources: &Arc<Mutex<HashMap<Position, (ResourceType, u32)>>>,
    tgt_pos: Position,
) -> Option<ResourceType> {
    let mut m = map.lock();
    if let Some(res) = m.resources.get_mut(&tgt_pos) {
        if res.quantity == 0 {
            shared_resources.lock().remove(&tgt_pos);
            return None;
        }

        res.quantity -= 1;
        let resource_type = res.resource_type;

        if res.quantity == 0 {
            m.resources.remove(&tgt_pos);
            m.cells[tgt_pos.y][tgt_pos.x] = CellType::Empty;
            shared_resources.lock().remove(&tgt_pos);
        } else {
            shared_resources
                .lock()
                .entry(tgt_pos)
                .and_modify(|entry| entry.1 = res.quantity);
        }

        Some(resource_type)
    } else {
        shared_resources.lock().remove(&tgt_pos);
        None
    }
}
