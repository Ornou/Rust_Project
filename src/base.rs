use crate::map::ResourceType;
use parking_lot::Mutex;
use std::sync::Arc;

pub struct Base {
    pub energy: Arc<Mutex<u32>>,
    pub crystals: Arc<Mutex<u32>>,
}

impl Base {
    pub fn new() -> Self {
        Base {
            energy: Arc::new(Mutex::new(0)),
            crystals: Arc::new(Mutex::new(0)),
        }
    }

    pub fn deposit_resource(&self, resource_type: ResourceType, quantity: u32) {
        match resource_type {
            ResourceType::Energy => {
                *self.energy.lock() += quantity;
            }
            ResourceType::Crystal => {
                *self.crystals.lock() += quantity;
            }
        }
    }

    pub fn get_total_energy(&self) -> u32 {
        *self.energy.lock()
    }

    pub fn get_total_crystals(&self) -> u32 {
        *self.crystals.lock()
    }
}

impl Default for Base {
    fn default() -> Self {
        Self::new()
    }
}
