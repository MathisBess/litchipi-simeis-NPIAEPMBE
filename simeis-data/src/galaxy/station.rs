use std::collections::BTreeMap;
use std::sync::Arc;

use mea::rwlock::RwLock;
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::crew::{Crew, CrewId, CrewMember, CrewMemberType};
use crate::errors::Errcode;
use crate::industry::{IndustryUnit, IndustryUnitId, IndustryUnitType};
use crate::market::{fee_rate, Market, MarketTx};
use crate::player::{Player, PlayerId};
use crate::ship::cargo::ShipCargo;
use crate::ship::module::ShipModuleId;
use crate::ship::resources::Resource;
use crate::ship::upgrade::ShipUpgrade;
use crate::ship::Ship;
use crate::utils::ShardedLockedData;

use super::scan::ScanResult;
use super::{Galaxy, SpaceCoord};

const CARGO_BASE_PRICE: f64 = 2.0;
const CARGO_PRICE_INCDIV: f64 = 100.0;
pub const STATION_INIT_CARGO: f64 = 1000.0;

pub type StationId = u16;

#[derive(Serialize, Deserialize, Debug)]
pub struct StationInfo {
    pub id: StationId,
    pub position: SpaceCoord,
}

impl StationInfo {
    // TODO (#8) Based on the scanner rank, get informations on crew and cargo
    pub fn scan(_rank: u8, station: &Station) -> StationInfo {
        StationInfo {
            id: station.id,
            position: station.position,
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct StationPlayerData {
    pub idle_crew: Crew,
    pub crew: Crew,
    pub trader: Option<CrewId>,
    pub cargo: ShipCargo,
    pub industry: BTreeMap<IndustryUnitId, IndustryUnit>,
}

impl StationPlayerData {
    pub fn new() -> StationPlayerData {
        StationPlayerData {
            cargo: ShipCargo::with_capacity(STATION_INIT_CARGO),
            ..Default::default()
        }
    }
}

pub struct Station {
    pub id: StationId,
    pub position: SpaceCoord,
    pub shipyard: RwLock<Vec<Ship>>,

    pub player_data: ShardedLockedData<PlayerId, Arc<RwLock<StationPlayerData>>>,
}

impl std::fmt::Debug for Station {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Station")
            .field("id", &self.id)
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl Station {
    pub fn init(id: u16, position: super::SpaceCoord) -> Station {
        Station {
            id,
            position,
            shipyard: RwLock::new(Ship::init_shipyard(position)),
            player_data: ShardedLockedData::new(100),
        }
    }

    // TODO (#8) Allow to build improvements for the scanner
    pub async fn scan(&self, galaxy: &Galaxy) -> ScanResult {
        galaxy.scan_sector(1, &self.position).await
    }

    pub async fn cargo_price(&self, player: &PlayerId) -> f64 {
        let cap = if let Some(data) = self.player_data.clone_val(player).await {
            data.read().await.cargo.capacity
        } else {
            STATION_INIT_CARGO
        };
        CARGO_BASE_PRICE.powf((cap - STATION_INIT_CARGO) / CARGO_PRICE_INCDIV)
    }

    pub async fn buy_industry(
        &self,
        player: &mut Player,
        unit: IndustryUnitType,
    ) -> Result<(IndustryUnitId, f64), Errcode> {
        let cost = unit.get_price_buy();
        if player.money < cost {
            return Err(Errcode::NotEnoughMoney(player.money, cost));
        }
        self.ensure_has_player_data(&player.id).await;
        let pd = self.player_data.clone_val(&player.id).await.unwrap();
        let mut pd = pd.write().await;
        let unit = unit.new_unit();
        let unit_id = unit.id;
        pd.industry.insert(unit_id, unit);
        player.money -= cost;
        Ok((unit_id, cost))
    }

    pub async fn upgrade_industry(
        &self,
        player: &mut Player,
        id: &IndustryUnitId,
    ) -> Result<u8, Errcode> {
        self.ensure_has_player_data(&player.id).await;
        let pd = self.player_data.clone_val(&player.id).await.unwrap();
        let mut pd = pd.write().await;
        let Some(unit) = pd.industry.get_mut(id) else {
            return Err(Errcode::NoSuchIndustryUnit);
        };
        let cost = unit.price_next_rank();
        if cost > player.money {
            return Err(Errcode::NotEnoughMoney(player.money, cost));
        }
        player.money -= cost;
        unit.rank += 1;
        Ok(unit.rank)
    }

    pub async fn start_industry(
        &self,
        player: &PlayerId,
        id: &IndustryUnitId,
    ) -> Result<(), Errcode> {
        self.ensure_has_player_data(player).await;
        let pd = self.player_data.clone_val(player).await.unwrap();
        let mut pd = pd.write().await;
        let Some(unit) = pd.industry.get_mut(id) else {
            return Err(Errcode::NoSuchIndustryUnit);
        };
        unit.started = true;
        Ok(())
    }

    pub async fn stop_industry(
        &self,
        player: &PlayerId,
        id: &IndustryUnitId,
    ) -> Result<(), Errcode> {
        self.ensure_has_player_data(player).await;
        let pd = self.player_data.clone_val(player).await.unwrap();
        let mut pd = pd.write().await;
        let Some(unit) = pd.industry.get_mut(id) else {
            return Err(Errcode::NoSuchIndustryUnit);
        };
        unit.started = false;
        Ok(())
    }

    pub async fn buy_cargo(&self, player: &mut Player, amnt: &usize) -> Result<ShipCargo, Errcode> {
        let cost = (*amnt as f64) * self.cargo_price(&player.id).await;
        if cost > player.money {
            return Err(Errcode::NotEnoughMoney(player.money, cost));
        }
        player.money -= cost;
        self.ensure_has_player_data(&player.id).await;
        let pd = self.player_data.clone_val(&player.id).await.unwrap();
        let mut pd = pd.write().await;
        pd.cargo.capacity += *amnt as f64;
        Ok(pd.cargo.clone())
    }

    pub async fn add_cargo_cap(&self, player: &PlayerId, amnt: usize) -> ShipCargo {
        self.ensure_has_player_data(player).await;
        let pd = self.player_data.clone_val(player).await.unwrap();
        let mut pd = pd.write().await;
        pd.cargo.capacity += amnt as f64;
        pd.cargo.clone()
    }

    pub async fn assign_trader(&self, pid: &PlayerId, id: CrewId) -> Result<(), Errcode> {
        self.ensure_has_player_data(pid).await;
        let pd = self.player_data.clone_val(pid).await.unwrap();
        let mut pd = pd.write().await;
        let Some(cm) = pd.idle_crew.0.remove(&id) else {
            return Err(Errcode::CrewMemberNotIdle(id));
        };

        pd.crew.0.insert(id, cm);
        pd.trader = Some(id);
        Ok(())
    }

    pub async fn onboard_pilot(&self, ship: &mut Ship, id: &CrewId) -> Result<(), Errcode> {
        self.ensure_has_player_data(&ship.owner).await;
        let pd = self.player_data.clone_val(&ship.owner).await.unwrap();
        let mut pd = pd.write().await;
        let Some(cm) = pd.idle_crew.0.get(id) else {
            return Err(Errcode::CrewMemberNotIdle(*id));
        };
        if cm.member_type != CrewMemberType::Pilot {
            return Err(Errcode::WrongCrewType(CrewMemberType::Pilot));
        }
        ship.pilot = Some(*id);
        let pilot = pd.idle_crew.0.remove(id).unwrap();
        ship.crew.0.insert(*id, pilot);
        ship.update_perf_stats();
        Ok(())
    }

    pub async fn onboard_operator(
        &self,
        ship: &mut Ship,
        id: &CrewId,
        mod_id: &ShipModuleId,
    ) -> Result<(), Errcode> {
        self.ensure_has_player_data(&ship.owner).await;
        let cm = self
            .get_idle_crew(&ship.owner, id, CrewMemberType::Operator)
            .await?;
        let Some(module) = ship.modules.get_mut(mod_id) else {
            return Err(Errcode::NoSuchModule(*mod_id));
        };
        if !module.need(&cm.member_type) {
            return Err(Errcode::CrewNotNeeded);
        }
        module.operator = Some(*id);
        let pd = self.player_data.clone_val(&ship.owner).await.unwrap();
        let mut pd = pd.write().await;
        ship.crew.0.insert(*id, pd.idle_crew.0.remove(id).unwrap());
        Ok(())
    }

    pub async fn assign_crew_to_industry(
        &self,
        pid: &PlayerId,
        id: &CrewId,
        iid: &IndustryUnitId,
    ) -> Result<(), Errcode> {
        let cm = self
            .get_idle_crew(pid, id, CrewMemberType::Operator)
            .await?;
        let pd = self.player_data.clone_val(pid).await.unwrap();
        let mut pd = pd.write().await;
        let Some(industry) = pd.industry.get_mut(iid) else {
            return Err(Errcode::NoSuchIndustryUnit);
        };
        if !industry.need_crew_member(&cm.member_type) {
            return Err(Errcode::CrewNotNeeded);
        }
        industry.assign_operator(*id, &cm);
        let cm = pd.idle_crew.0.remove(id).unwrap();
        pd.crew.0.insert(*id, cm);
        Ok(())
    }

    pub async fn get_idle_crew(
        &self,
        pid: &PlayerId,
        id: &CrewId,
        ctype: CrewMemberType,
    ) -> Result<CrewMember, Errcode> {
        self.ensure_has_player_data(pid).await;
        let pd = self.player_data.clone_val(pid).await.unwrap();
        let pd = pd.read().await;
        let Some(cm) = pd.idle_crew.0.get(id) else {
            return Err(Errcode::CrewMemberNotIdle(*id));
        };
        if cm.member_type != ctype {
            return Err(Errcode::WrongCrewType(ctype));
        }
        Ok(cm.clone())
    }

    pub async fn buy_resource(
        &self,
        market: &Market,
        player: &PlayerId,
        resource: &Resource,
        amnt: f64,
    ) -> Result<MarketTx, Errcode> {
        self.ensure_has_player_data(player).await;
        let pd = self.player_data.clone_val(player).await.unwrap();
        let mut pd = pd.write().await;
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        let can_cargo = pd.cargo.space_for(resource);
        let amnt = amnt.min(can_cargo);
        if amnt == 0.0 {
            return Err(Errcode::BuyNothing);
        }
        let tx = market.buy(cm, resource, amnt).await;
        let (r, a) = tx.added_cargo.unwrap();
        pd.cargo.add_resource(&r, a);
        Ok(tx)
    }

    pub async fn sell_resource(
        &self,
        market: &Market,
        player: &PlayerId,
        resource: &Resource,
        amnt: f64,
    ) -> Result<MarketTx, Errcode> {
        self.ensure_has_player_data(player).await;
        let pd = self.player_data.clone_val(player).await.unwrap();
        let mut pd = pd.write().await;
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        let Some(can_cargo) = pd.cargo.resources.get(resource) else {
            return Err(Errcode::SellNothing);
        };
        let amnt = amnt.min(*can_cargo);
        if amnt <= 0.0 {
            return Err(Errcode::SellNothing);
        }
        let tx = market.sell(cm, resource, amnt).await;
        let (r, a) = tx.removed_cargo.unwrap();
        let unloaded = pd.cargo.unload(&r, a);
        debug_assert_eq!(unloaded, a);
        Ok(tx)
    }

    pub async fn refuel_ship(&self, ship: &mut Ship) -> Result<f64, Errcode> {
        if self.position != ship.position {
            return Err(Errcode::ShipNotInStation);
        }
        let Some(pd) = self.player_data.clone_val(&ship.owner).await else {
            return Err(Errcode::NoFuelInCargo);
        };
        let mut pd = pd.write().await;
        let Some(qty) = pd.cargo.resources.get(&Resource::Fuel) else {
            return Err(Errcode::NoFuelInCargo);
        };
        if *qty == 0.0 {
            return Err(Errcode::NoFuelInCargo);
        }
        debug_assert!(ship.fuel_tank >= 0.0);
        debug_assert!(ship.fuel_tank_capacity >= ship.fuel_tank);
        let needed = ship.fuel_tank_capacity - ship.fuel_tank;
        let unload = needed.min(*qty);
        let unloaded = pd.cargo.unload(&Resource::Fuel, unload);
        ship.fuel_tank += unloaded;
        debug_assert!(ship.fuel_tank_capacity >= ship.fuel_tank);
        Ok(unloaded)
    }

    pub async fn repair_ship(&self, ship: &mut Ship) -> Result<f64, Errcode> {
        if self.position != ship.position {
            return Err(Errcode::ShipNotInStation);
        }
        let Some(pd) = self.player_data.clone_val(&ship.owner).await else {
            return Err(Errcode::NoHullInCargo);
        };
        let mut pd = pd.write().await;
        let Some(qty) = pd.cargo.resources.get(&Resource::Hull) else {
            return Err(Errcode::NoHullInCargo);
        };
        if *qty == 0.0 {
            return Err(Errcode::NoHullInCargo);
        }
        debug_assert!(ship.hull_resistance >= ship.hull_decay);

        let amnt = ship.hull_decay.min(*qty);
        if amnt == 0.0 {
            return Ok(0.0);
        }
        let unloaded = pd.cargo.unload(&Resource::Hull, amnt);
        ship.hull_decay -= unloaded;
        debug_assert!(
            ship.hull_resistance >= ship.hull_decay,
            "{} < {}",
            ship.hull_resistance,
            ship.hull_decay
        );
        debug_assert!(ship.hull_decay >= 0.0, "{}", ship.hull_decay);
        debug_assert!(unloaded >= 0.0, "{}", unloaded);
        Ok(unloaded)
    }

    pub fn get_ship_upgrade_price(&self, _ship: &Ship, upgrade: &ShipUpgrade) -> f64 {
        // TODO (#9) Modify price based on station economy metrics
        // TODO (#9) Modify price based on upgrades already installed on the ship
        upgrade.get_price()
    }

    pub async fn get_cargo_potential_price(&self, id: &PlayerId) -> f64 {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return 0.0;
        };
        let pd = pd.read().await;
        pd.cargo
            .resources
            .iter()
            .map(|(r, amnt)| r.base_price() * amnt)
            .sum()
    }

    pub async fn add_resource(&self, id: &PlayerId, resource: &Resource, amnt: f64) -> f64 {
        self.ensure_has_player_data(id).await;
        let pd = self.player_data.clone_val(id).await.unwrap();
        let mut pd = pd.write().await;
        pd.cargo.add_resource(resource, amnt)
    }

    pub async fn buy_ship(&self, index: usize) -> Ship {
        // Ship starters, always keep them
        let mut ship = if index < 3 {
            let shipyard = self.shipyard.read().await;
            shipyard.get(index).unwrap().clone()
        } else {
            let mut shipyard = self.shipyard.write().await;
            let ship = shipyard.remove(index);
            shipyard.push(Ship::random(self.position));
            ship
        };
        ship.update_perf_stats();
        ship.fuel_tank = ship.fuel_tank_capacity;
        ship
    }

    pub async fn ensure_has_player_data(&self, id: &PlayerId) {
        if !self.player_data.contains_key(id).await {
            let pd = Arc::new(RwLock::new(StationPlayerData::new()));
            self.player_data.insert(*id, pd).await;
        }
    }

    pub async fn sum_all_wages(&self, id: &PlayerId) -> f64 {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return 0.0;
        };
        let pd = pd.read().await;
        pd.crew.sum_wages() + pd.idle_crew.sum_wages()
    }

    pub async fn upgrade_station_crew(
        &self,
        id: &PlayerId,
        money: &mut f64,
        crew: &CrewId,
    ) -> Result<(f64, u8), Errcode> {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return Err(Errcode::CrewMemberNotFound(*crew));
        };
        let mut pd = pd.write().await;

        let Some(cm) = pd.crew.0.get_mut(crew) else {
            return Err(Errcode::CrewMemberNotFound(*crew));
        };
        let price = cm.price_next_rank();
        if price > *money {
            return Err(Errcode::NotEnoughMoney(*money, price));
        }
        *money -= price;
        cm.rank += 1;
        Ok((price, cm.rank))
    }

    pub async fn hire_crew(&self, id: &PlayerId, crewtype: CrewMemberType) -> CrewId {
        let crewid = rand::rng().random();
        let member = CrewMember::from(crewtype);

        self.ensure_has_player_data(id).await;
        let pd = self.player_data.clone_val(id).await.unwrap();
        let mut pd = pd.write().await;
        pd.idle_crew.0.insert(crewid, member);
        crewid
    }

    pub async fn upgr_trader_price(&self, id: &PlayerId) -> Option<f64> {
        let pd = self.player_data.clone_val(id).await?;
        let pd = pd.read().await;
        pd.trader.map(|trader| {
            let cm = pd.crew.0.get(&trader).unwrap();
            cm.price_next_rank()
        })
    }

    pub async fn clone_cargo(&self, id: &PlayerId) -> ShipCargo {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return ShipCargo::with_capacity(STATION_INIT_CARGO);
        };
        let pd = pd.read().await;
        pd.cargo.clone()
    }

    pub async fn get_fee_rate(&self, id: &PlayerId) -> Result<f64, Errcode> {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return Err(Errcode::NoTraderAssigned);
        };
        let pd = pd.read().await;
        let Some(trader) = pd.trader else {
            return Err(Errcode::NoTraderAssigned);
        };
        let cm = pd.crew.0.get(&trader).unwrap();
        Ok(fee_rate(cm.rank))
    }

    pub async fn to_json(&self, id: &PlayerId) -> serde_json::Value {
        if let Some(pd) = self.player_data.clone_val(id).await {
            let pd = pd.read().await;
            self._to_json(&pd)
        } else {
            let pd = StationPlayerData::new();
            self._to_json(&pd)
        }
    }

    fn _to_json(&self, data: &StationPlayerData) -> serde_json::Value {
        serde_json::json!({
            "id": self.id,
            "position": self.position,
            "crew": data.crew,
            "cargo": data.cargo,
            "idle_crew": data.idle_crew,
            "trader": data.trader,
        })
    }

    pub async fn update_crafting(&self, tdelta: f64, id: &PlayerId) {
        let Some(pd) = self.player_data.clone_val(id).await else {
            return;
        };
        let mut pd = pd.write().await;
        let all_industry = pd.industry.clone();
        for (_, industry) in all_industry.iter() {
            if let Some(ratio) = industry.can_work(&tdelta, &pd.cargo.resources) {
                let t = tdelta * ratio;
                industry.work(t, &mut pd.cargo.resources);
            }
        }
    }

    pub async fn get_industry_production(
        &self,
        pid: &PlayerId,
        id: IndustryUnitId,
    ) -> Result<(Vec<(Resource, f64)>, Vec<(Resource, f64)>), Errcode> {
        let Some(pd) = self.player_data.clone_val(pid).await else {
            return Err(Errcode::NoSuchIndustryUnit);
        };

        let pd = pd.read().await;
        let Some(industry) = pd.industry.get(&id) else {
            return Err(Errcode::NoSuchIndustryUnit);
        };

        let Some(opid) = industry.operator else {
            return Ok((vec![], vec![]));
        };
        let op = pd.crew.0.get(&opid).unwrap();

        let inputs = industry.input(op.rank);
        let outputs = industry.output(op.rank);
        Ok((inputs, outputs))
    }
}
