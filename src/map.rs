use noise::{NoiseFn, Perlin};
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

impl Position {
    pub fn new(x: usize, y: usize) -> Self {
        Position { x, y }
    }

    pub fn distance_to(&self, other: &Position) -> f64 {
        let dx = (self.x as i32 - other.x as i32) as f64;
        let dy = (self.y as i32 - other.y as i32) as f64;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn neighbors(&self, width: usize, height: usize) -> Vec<Position> {
        let mut neighbors = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = (self.x as i32 + dx) as usize;
                let ny = (self.y as i32 + dy) as usize;
                if nx < width && ny < height {
                    neighbors.push(Position::new(nx, ny));
                }
            }
        }
        neighbors
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellType {
    Empty,
    Obstacle,
    Energy,
    Crystal,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Energy,
    Crystal,
}

pub struct Map {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<Vec<CellType>>,
    pub resources: HashMap<Position, Resource>,
    pub base_position: Position,
}

impl Map {
    pub fn generate(width: usize, height: usize) -> Self {
        let mut rng = rand::thread_rng();
        let perlin = Perlin::new(rng.gen());

        // Generate obstacles using Perlin noise
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

        // Place base in the center
        let base_x = width / 2;
        let base_y = height / 2;
        cells[base_y][base_x] = CellType::Empty;

        // Place resources
        let mut resources = HashMap::new();
        let num_resources = 15;

        for _ in 0..num_resources {
            let mut x;
            let mut y;
            loop {
                x = rng.gen_range(0..width);
                y = rng.gen_range(0..height);
                if cells[y][x] == CellType::Empty {
                    break;
                }
            }

            let pos = Position::new(x, y);
            let resource_type = if rng.gen_bool(0.5) {
                ResourceType::Energy
            } else {
                ResourceType::Crystal
            };
            let quantity = rng.gen_range(50..200);

            cells[y][x] = match resource_type {
                ResourceType::Energy => CellType::Energy,
                ResourceType::Crystal => CellType::Crystal,
            };

            resources.insert(pos, Resource {
                resource_type,
                quantity,
            });
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

    pub fn is_walkable(&self, pos: Position) -> bool {
        matches!(
            self.get_cell(pos),
            CellType::Empty | CellType::Energy | CellType::Crystal
        )
    }
}
