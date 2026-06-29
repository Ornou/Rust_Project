use crate::map::ResourceType;
use parking_lot::Mutex;

struct BaseState {
    energy: u32,
    crystals: u32,
}

pub struct Base {
    state: Mutex<BaseState>,
}

impl Base {
    pub fn new() -> Self {
        Base {
            state: Mutex::new(BaseState {
                energy: 0,
                crystals: 0,
            }),
        }
    }

    pub fn deposit_resource(&self, resource_type: ResourceType, quantity: u32) {
        let mut inner = self.state.lock();
        match resource_type {
            ResourceType::Energy => {
                inner.energy += quantity;
            }
            ResourceType::Crystal => {
                inner.crystals += quantity;
            }
        }
    }

    pub fn get_total_energy(&self) -> u32 {
        self.state.lock().energy
    }

    pub fn get_total_crystals(&self) -> u32 {
        self.state.lock().crystals
    }
}

impl Default for Base {
    fn default() -> Self {
        Self::new()
    }
}
