use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::resources::Resource;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShipCargo {
    pub capacity: f64,
    pub usage: f64,
    pub resources: BTreeMap<Resource, f64>,
}

impl ShipCargo {
    pub const fn with_capacity(cap: f64) -> ShipCargo {
        ShipCargo {
            usage: 0.0,
            capacity: cap,
            resources: BTreeMap::new(),
        }
    }

    pub fn slowing_ratio(&self) -> f64 {
        // let usage_ratio = self.usage / self.capacity;
        // TODO (#11)    Cargo slows down speed of ship
        0.0
    }

    pub fn add_resource(&mut self, res: &Resource, mut amnt: f64) -> f64 {
        let added = res.volume() * amnt;
        if self.usage == self.capacity {
            return 0.0;
        } else if (self.usage + added) > self.capacity {
            let overflow = (self.usage + added) - self.capacity;
            amnt -= overflow / res.volume();
            self.usage = self.capacity;
        } else {
            self.usage += added;
        }

        if let Some(stock) = self.resources.get_mut(res) {
            *stock += amnt;
        } else {
            self.resources.insert(*res, amnt);
        }
        amnt
    }

    pub fn is_full(&self) -> bool {
        self.usage == self.capacity
    }

    pub fn unload(&mut self, resource: &Resource, amnt: f64) -> f64 {
        if let Some(got) = self.resources.get_mut(resource) {
            let unload = got.min(amnt);
            *got -= unload;
            self.usage = (self.usage - (resource.volume() * unload)).max(0.0);
            self.usage = (self.usage * 1000.0).round() / 1000.0;
            unload
        } else {
            0.0
        }
    }

    // Compute how much of a resource we can store (based on its volume)
    pub fn space_for(&self, resource: &Resource) -> f64 {
        let capleft = self.capacity - self.usage;
        capleft / resource.volume()
    }
}

#[test]
fn test_cargo_overflow() {
    let mut cargo = ShipCargo::with_capacity(100.0 * Resource::Carbon.volume());
    let added = cargo.add_resource(&Resource::Carbon, 95.0);
    assert_eq!(added, 95.0);

    let added = cargo.add_resource(&Resource::Carbon, 10.0);
    assert_eq!(added, 5.0);
    assert_eq!(cargo.usage, cargo.capacity);
}
