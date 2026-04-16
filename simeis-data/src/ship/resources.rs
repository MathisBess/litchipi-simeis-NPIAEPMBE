use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::galaxy::planet::Planet;

use super::{cargo::ShipCargo, Ship};

#[derive(
    EnumIter,
    EnumString,
    IntoStaticStr,
    Debug,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Clone,
    Copy,
)]
#[strum(ascii_case_insensitive)]
pub enum Resource {
    // Solid
    Carbon,
    Iron,
    Copper,
    Gold,

    // Liquid
    Water,
    Alcohol,
    Oil,
    SulfuricAcid,

    // Gaseous
    Hydrogen,
    Oxygen,
    Helium,
    Ozone,

    // Crafted
    Fuel,
    Hull,
}

impl Resource {
    pub fn scored(&self) -> bool {
        matches!(self, Resource::Fuel | Resource::Hull)
    }

    // TODO (#12) Get from configuration file
    #[inline]
    pub const fn base_price(&self) -> f64 {
        let base = 4.0;
        match self {
            Resource::Carbon | Resource::Hydrogen | Resource::Water => base,
            Resource::Iron | Resource::Oxygen | Resource::Alcohol => 4.0 * base,
            Resource::Copper | Resource::Helium | Resource::Oil => 12.0 * base,
            Resource::Gold | Resource::Ozone | Resource::SulfuricAcid => 16.0 * base,
            Resource::Fuel | Resource::Hull => base / 2.0,
        }
    }

    pub fn volume(&self) -> f64 {
        match self {
            Resource::Carbon | Resource::Hydrogen | Resource::Water => 0.75,
            Resource::Iron | Resource::Oxygen | Resource::Alcohol => 2.5,
            Resource::Copper | Resource::Helium | Resource::Oil => 3.0,
            Resource::Gold | Resource::Ozone | Resource::SulfuricAcid => 0.25,
            Resource::Fuel => 2.0,
            Resource::Hull => 0.05,
        }
    }

    #[allow(unused_mut)]
    pub fn extraction_difficulty(&self) -> f64 {
        let mut base = 0.25;

        #[cfg(feature = "extraspeed")]
        {
            base /= 1000.0;
        }
        match self {
            Resource::Carbon | Resource::Hydrogen => base,
            Resource::Iron | Resource::Oxygen => 3.75 * base,
            Resource::Copper | Resource::Helium => 11.0 * base,
            Resource::Gold | Resource::Ozone => 14.0 * base,

            // All the things that are only crafted
            _ => unreachable!("Extraction difficulty on crafted resources"),
        }
    }

    pub fn min_rank(&self) -> u8 {
        match self {
            Resource::Carbon | Resource::Hydrogen | Resource::Water => 0,
            Resource::Iron | Resource::Oxygen | Resource::Alcohol => 2,
            Resource::Copper | Resource::Helium | Resource::Oil => 4,
            Resource::Gold | Resource::Ozone | Resource::SulfuricAcid => 6,
            Resource::Fuel | Resource::Hull => 0,
        }
    }

    pub fn mineable(&self, rank: u8) -> bool {
        match self {
            Resource::Carbon | Resource::Iron | Resource::Copper | Resource::Gold => {
                rank > self.min_rank()
            }
            _ => false,
        }
    }

    pub fn suckable(&self, rank: u8) -> bool {
        match self {
            Resource::Hydrogen | Resource::Oxygen | Resource::Helium | Resource::Ozone => {
                rank > self.min_rank()
            }
            _ => false,
        }
    }

    pub fn pumpable(&self, rank: u8) -> bool {
        match self {
            Resource::Water | Resource::Alcohol | Resource::Oil | Resource::SulfuricAcid => {
                rank > self.min_rank()
            }
            _ => false,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExtractionInfo {
    pub mining_rate: BTreeMap<Resource, f64>,
    pub time_fill_cargo: f64,
}

impl ExtractionInfo {
    pub fn create(ship: &Ship, planet: &Planet) -> Self {
        let mut extraction = BTreeMap::new();
        let mut cap_per_sec = 0.0;
        for (_, smod) in ship.modules.iter() {
            for (res, rate) in smod.can_extract(&ship.crew, planet) {
                cap_per_sec += rate * res.volume();
                if let Some(rrate) = extraction.get_mut(&res) {
                    *rrate += rate;
                } else {
                    extraction.insert(res, rate);
                }
            }
        }
        ExtractionInfo {
            mining_rate: extraction,
            time_fill_cargo: ship.cargo.capacity / cap_per_sec,
        }
    }

    pub fn update_cargo(&self, cargo: &mut ShipCargo, tdelta: f64) -> bool {
        for (res, rate) in self.mining_rate.iter() {
            cargo.add_resource(res, *rate * tdelta);
        }
        cargo.is_full()
    }
}
