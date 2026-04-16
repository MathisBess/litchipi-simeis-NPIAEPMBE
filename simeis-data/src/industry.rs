use std::collections::BTreeMap;

use rand::RngExt;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

use crate::{
    crew::{CrewId, CrewMember, CrewMemberType},
    ship::resources::Resource,
};

pub type IndustryUnitId = u32;

const UNIT_UPG_POWF_DIV: f64 = 75.0;

// TODO (#12) Get from configuration file
const SBASE_REQ: f64 = 1.5;
const ABASE_REQ: f64 = 7.5;

// Because all resources of the same level have the same base price
// The resource cost (in credits) should be the same whatever the unit is
// As long as it's the same class (simple / advanced)
pub const fn get_simple_industry_resources_cost() -> f64 {
    (SBASE_REQ * Resource::Hydrogen.base_price())
        + (SBASE_REQ * 0.2 * Resource::Oxygen.base_price())
        + (SBASE_REQ * 1.25 * Resource::Carbon.base_price())
        + (SBASE_REQ * 0.4 * Resource::Water.base_price())
}

pub const fn get_advanced_industry_resources_cost() -> f64 {
    (ABASE_REQ * Resource::Carbon.base_price())
        + (ABASE_REQ * 0.4 * Resource::Oil.base_price())
        + (ABASE_REQ * 0.2 * Resource::Helium.base_price())
}

pub const fn get_sbase_produce_base() -> f64 {
    let scost = get_simple_industry_resources_cost();
    scost / (1.05 * Resource::Fuel.base_price())
}

pub const fn get_abase_produce_base() -> f64 {
    let acost = get_advanced_industry_resources_cost();
    acost / (1.75 * Resource::Fuel.base_price())
}

#[derive(
    EnumIter,
    EnumString,
    IntoStaticStr,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[strum(ascii_case_insensitive)]
pub enum IndustryUnitType {
    SimpleFuelRefinery,
    AdvancedFuelRefinery,

    SimpleHullFoundry,
    AdvancedHullFoundry,
}

impl IndustryUnitType {
    pub fn new_unit(self) -> IndustryUnit {
        let unitid = rand::rng().random();
        IndustryUnit {
            id: unitid,
            operator: None,
            unittype: self,
            rank: 1,
            started: false,
            resources_required: vec![],
            resources_created: vec![],
        }
    }

    #[inline]
    // TODO (#9) Move price based on inflation rate of the station
    pub fn get_price_buy(&self) -> f64 {
        match self {
            IndustryUnitType::SimpleHullFoundry | IndustryUnitType::SimpleFuelRefinery => 8000.0,
            IndustryUnitType::AdvancedHullFoundry | IndustryUnitType::AdvancedFuelRefinery => {
                18000.0
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IndustryUnit {
    pub id: IndustryUnitId,
    pub unittype: IndustryUnitType,
    pub rank: u8,
    pub started: bool,

    pub operator: Option<CrewId>,
    resources_required: Vec<(Resource, f64)>,
    resources_created: Vec<(Resource, f64)>,
}

impl IndustryUnit {
    #[inline]
    pub fn price_next_rank(&self) -> f64 {
        let num = UNIT_UPG_POWF_DIV - 1.0 + (self.rank as f64);
        self.unittype.get_price_buy().powf(num / UNIT_UPG_POWF_DIV)
    }

    #[inline]
    pub fn need_crew_member(&self, ctype: &CrewMemberType) -> bool {
        ctype == &CrewMemberType::Operator && self.operator.is_none()
    }

    #[inline]
    pub fn assign_operator(&mut self, opid: CrewId, op: &CrewMember) {
        self.operator = Some(opid);
        self.new_op_rank(op.rank);
    }

    #[inline]
    pub fn new_op_rank(&mut self, rank: u8) {
        self.resources_required = self.input(rank);
        self.resources_created = self.output(rank);
    }

    #[inline]
    pub fn input(&self, oprank: u8) -> Vec<(Resource, f64)> {
        debug_assert_ne!(oprank, 0);
        let div = 1.0 / (std::f64::consts::E + (oprank as f64) - 1.0).ln();
        match self.unittype {
            IndustryUnitType::SimpleFuelRefinery => {
                let sbase = SBASE_REQ * (self.rank as f64);
                vec![
                    (Resource::Hydrogen, sbase),      // Gas 1
                    (Resource::Oxygen, sbase * 0.2),  // Gas 2
                    (Resource::Carbon, sbase * 1.25), // Solid 1
                    (Resource::Water, sbase * 0.4),   // Liquid 1
                ]
            }
            IndustryUnitType::SimpleHullFoundry => {
                let sbase = SBASE_REQ * (self.rank as f64);
                vec![
                    (Resource::Carbon, sbase),          // Solid 1
                    (Resource::Iron, sbase * 0.2),      // Solid 2
                    (Resource::Hydrogen, sbase * 1.25), // Gas 1
                    (Resource::Water, 0.5 * 0.4),       // Liquid 1
                ]
            }
            IndustryUnitType::AdvancedFuelRefinery => {
                let abase = ABASE_REQ * (self.rank as f64);
                vec![
                    (Resource::Carbon, abase),       // Solid 1
                    (Resource::Oil, abase * 0.4),    // Liquid 3
                    (Resource::Helium, abase * 0.2), // Gas 3
                ]
            }
            IndustryUnitType::AdvancedHullFoundry => {
                let abase = ABASE_REQ * (self.rank as f64);
                vec![
                    (Resource::Hydrogen, abase),     // Gas 1
                    (Resource::Copper, abase * 0.4), // Solid 3
                    (Resource::Oil, abase * 0.2),    // Liquid 3
                ]
            }
        }
        .into_iter()
        .map(|(res, amnt)| {
            let amnt: f64 = amnt;
            let new_amnt = amnt.powf(div);
            (res, new_amnt)
        })
        .collect()
    }

    #[inline]
    pub fn output(&self, oprank: u8) -> Vec<(Resource, f64)> {
        debug_assert_ne!(oprank, 0);
        let pown = (oprank as f64).ln();

        match self.unittype {
            IndustryUnitType::SimpleFuelRefinery => vec![(
                Resource::Fuel,
                get_sbase_produce_base() * (self.rank as f64),
            )],
            IndustryUnitType::SimpleHullFoundry => vec![(
                Resource::Hull,
                get_sbase_produce_base() * (self.rank as f64),
            )],
            IndustryUnitType::AdvancedFuelRefinery => vec![(
                Resource::Fuel,
                get_abase_produce_base() * (self.rank as f64),
            )],
            IndustryUnitType::AdvancedHullFoundry => vec![(
                Resource::Hull,
                get_abase_produce_base() * (self.rank as f64),
            )],
        }
        .into_iter()
        .map(|(res, amnt)| {
            let amnt: f64 = amnt;
            (res, amnt.powf(pown))
        })
        .collect()
    }

    pub fn can_work(&self, tdelta: &f64, resources: &BTreeMap<Resource, f64>) -> Option<f64> {
        if !self.started {
            return None;
        }
        self.operator?;
        let mut max_ratio: f64 = 0.0;
        for (res, amnt) in self.resources_required.iter() {
            if let Some(incargo) = resources.get(res) {
                let max = amnt * tdelta;
                if incargo >= &max {
                    max_ratio = 1.0;
                } else {
                    let ratio = incargo / max;
                    max_ratio = max_ratio.max(ratio);
                }
            } else {
                return None;
            }
        }
        Some(max_ratio)
    }

    pub fn work(&self, tdelta: f64, resources: &mut BTreeMap<Resource, f64>) {
        debug_assert!(self.started);
        debug_assert!(self.operator.is_some());
        for (res, amnt) in self.resources_required.iter() {
            let n = resources.get_mut(res).unwrap();
            *n -= amnt * tdelta;
            log::warn!("Used {} of {res:?}, got {n} left", amnt * tdelta);
        }

        for (res, amnt) in self.resources_created.iter() {
            if !resources.contains_key(res) {
                resources.insert(*res, 0.0);
            }

            let n = resources.get_mut(res).unwrap();
            *n += amnt * tdelta;
            log::warn!("Created {} of {res:?}, got {n} now", amnt * tdelta);
        }
    }
}
