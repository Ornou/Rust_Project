use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Position { x, y }
    }

    pub fn neighbors_cardinal(&self, width: usize, height: usize) -> Vec<Position> {
        let dirs: &[(i32, i32)] = &[(0, -1), (0, 1), (-1, 0), (1, 0)];
        let mut out = Vec::new();
        for (dx, dy) in dirs {
            let nx = self.x as i32 + dx;
            let ny = self.y as i32 + dy;
            if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                out.push(Position::new(nx as usize, ny as usize));
            }
        }
        out
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Empty,
    Obstacle,
    Energy,
    Crystal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Energy,
    Crystal,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub quantity: u32,
}

#[derive(Clone)]
pub struct Map {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<CellType>>,
    pub resources: HashMap<Position, Resource>,
    pub base_position: Position,
}

impl Map {
    pub fn generate<R: Rng + ?Sized>(width: usize, height: usize, rng: &mut R) -> Self {
        let perlin = Perlin::new(rng.gen());

        let mut cells = vec![vec![CellType::Empty; width]; height];
        for y in 0..height {
            for x in 0..width {
                let nx = x as f64 / width as f64;
                let ny = y as f64 / height as f64;
                let noise_val = perlin.get([nx * 5.0, ny * 5.0, 0.0]);
                if noise_val > 0.3 {
                    cells[y][x] = CellType::Obstacle;
                }
            }
        }

        // Clear a 3x3 area around the base so robots can always spawn and return
        let base_x = width / 2;
        let base_y = height / 2;
        for dy in -1i32..=1 {
            for dx in -1i32..=1 {
                let nx = (base_x as i32 + dx) as usize;
                let ny = (base_y as i32 + dy) as usize;
                if nx < width && ny < height {
                    cells[ny][nx] = CellType::Empty;
                }
            }
        }

        let mut resources = HashMap::new();
        let num_resources = 20;

        for _ in 0..num_resources {
            let mut attempts = 0;
            loop {
                attempts += 1;
                if attempts > 2000 {
                    break;
                }
                let x = rng.gen_range(0..width);
                let y = rng.gen_range(0..height);
                let is_near_base =
                    (x as i32 - base_x as i32).abs() <= 3 && (y as i32 - base_y as i32).abs() <= 3;
                if cells[y][x] == CellType::Empty && !is_near_base {
                    let resource_type = if rng.gen_bool(0.5) {
                        ResourceType::Energy
                    } else {
                        ResourceType::Crystal
                    };
                    let quantity = rng.gen_range(50..=200);
                    cells[y][x] = match resource_type {
                        ResourceType::Energy => CellType::Energy,
                        ResourceType::Crystal => CellType::Crystal,
                    };
                    resources.insert(
                        Position::new(x, y),
                        Resource {
                            resource_type,
                            quantity,
                        },
                    );
                    break;
                }
            }
        }

        Map {
            width,
            height,
            cells,
            resources,
            base_position: Position::new(base_x, base_y),
        }
    }

    pub fn get_cell(&self, pos: Position) -> CellType {
        if pos.x < self.width && pos.y < self.height {
            self.cells[pos.y][pos.x]
        } else {
            CellType::Obstacle
        }
    }
}

/// BFS returning the first step from `from` toward `to`, avoiding Obstacle cells.
pub fn bfs_next_step(map: &Map, from: Position, to: Position) -> Option<Position> {
    if from == to {
        return None;
    }

    let mut queue = VecDeque::new();
    let mut visited: HashSet<Position> = HashSet::new();
    let mut parent: HashMap<Position, Position> = HashMap::new();

    queue.push_back(from);
    visited.insert(from);

    while let Some(current) = queue.pop_front() {
        if current == to {
            // Walk back through parents to find the first step
            let mut step = to;
            loop {
                match parent.get(&step) {
                    Some(&p) if p == from => return Some(step),
                    Some(&p) => step = p,
                    None => return None,
                }
            }
        }

        for neighbor in current.neighbors_cardinal(map.width, map.height) {
            if !visited.contains(&neighbor) {
                // Allow passing through resource cells and the target even if it's an obstacle
                if map.get_cell(neighbor) != CellType::Obstacle || neighbor == to {
                    visited.insert(neighbor);
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }
    }

    None
}
