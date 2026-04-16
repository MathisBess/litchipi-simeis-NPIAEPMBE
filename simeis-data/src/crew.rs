use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use strum::{EnumString, IntoStaticStr};

const WAGE_INC_RANK_POWF: f64 = 0.85;
const RANK_PRICE_WAGE_MULT: f64 = 1900.0;

pub type CrewId = u32;

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
pub struct Crew(pub BTreeMap<CrewId, CrewMember>);
impl Crew {
    pub fn sum_wages(&self) -> f64 {
        self.0.values().map(|crew| crew.wage()).sum::<f64>()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CrewMember {
    pub member_type: CrewMemberType,
    pub rank: u8,
}

impl From<CrewMemberType> for CrewMember {
    fn from(member_type: CrewMemberType) -> Self {
        CrewMember {
            member_type,
            rank: 1,
        }
    }
}

impl CrewMember {
    pub fn wage(&self) -> f64 {
        let op = 0.75;
        let base = match self.member_type {
            CrewMemberType::Operator => op,
            CrewMemberType::Pilot => op * 8.0,
            CrewMemberType::Trader => op * 5.0,
            CrewMemberType::Soldier => op * 2.5,
        };
        base * (self.rank as f64).powf(WAGE_INC_RANK_POWF)
    }

    #[inline]
    pub fn price_next_rank(&self) -> f64 {
        self.wage().powf(1.75) * RANK_PRICE_WAGE_MULT
    }
}

#[allow(dead_code)]
#[derive(EnumString, IntoStaticStr, Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[strum(ascii_case_insensitive)]
pub enum CrewMemberType {
    Pilot,
    Operator,
    Trader,
    Soldier,
}
