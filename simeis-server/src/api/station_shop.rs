use std::collections::BTreeMap;
use std::str::FromStr;

use ntex::router::IntoPattern;
use ntex::web;
use ntex::web::scope;
use ntex::web::types::Path;
use ntex::web::HttpRequest;
use ntex::web::ServiceConfig;

use serde_json::json;
use serde_json::to_value;
use strum::IntoEnumIterator;

use simeis_data::errors::Errcode;
use simeis_data::galaxy::station::StationId;
use simeis_data::ship::module::ShipModuleId;
use simeis_data::ship::module::ShipModuleType;
use simeis_data::ship::ShipId;

use crate::api::build_response;
use crate::api::GameState;

// @summary List all the modules available to buy on the station
// @returns For each module, its price
#[web::get("/modules")]
async fn get_prices_ship_module(
    srv: GameState,
    id: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let station_id = *id;
    // We need to ensure the station exist, even if we don't use it here
    let data = srv
        .map_station(&pkey, &station_id, |_, _| {
            Box::pin(async move {
                let mut res: BTreeMap<ShipModuleType, f64> = BTreeMap::new();
                for smod in ShipModuleType::iter() {
                    let price = smod.get_price_buy();
                    res.insert(smod, price);
                }
                Ok(to_value(res).unwrap())
            })
        })
        .await;
    build_response(data)
}

// @summary Buy a ship module and install it on a ship
// @returns The ID of the module, and the cost of the operation
#[web::post("/modules/{ship_id}/buy/{modtype}")]
async fn buy_ship_module(
    srv: GameState,
    args: Path<(StationId, ShipId, String)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, ship_id, modtype) = args.clone();
    let Ok(modtype) = ShipModuleType::from_str(modtype.as_str()) else {
        return build_response(Err(Errcode::InvalidArgument("modtype")));
    };
    let cost = modtype.get_price_buy();

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                player
                    .buy_ship_module(&station_id, &ship_id, modtype)
                    .await
                    .map(|v| json!({ "id": v, "cost": cost }))
            })
        })
        .await;
    build_response(data)
}

// @summary List the available upgrades for a module on a ship
// @returns For each module, the type of module, and price of the next upgrade
#[web::get("/modules/{ship_id}/upgrade")]
async fn get_ship_module_upgrade_prices(
    srv: GameState,
    args: Path<(StationId, ShipId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, ship_id) = *args;

    let data = srv
        .map_ship_in_station(&pkey, &station_id, &ship_id, |_, _, ship| {
            Box::pin(async move {
                let mut res = BTreeMap::new();
                for (id, module) in ship.modules.iter() {
                    res.insert(
                        id,
                        json!({
                            "module-type": module.modtype.clone(),
                            "price": module.price_next_rank(),
                        }),
                    );
                }
                Ok(to_value(res).unwrap())
            })
        })
        .await;
    build_response(data)
}

// @summary Buy an upgrade for a module installed on a ship
// @returns The new rank of the module, and the cost of the upgrade
// The level of a module will affect the extraction rate for a resource, as well as what kind of resources it kind mine.
#[web::post("/modules/{ship_id}/upgrade/{mod_id}")]
async fn buy_ship_module_upgrade(
    srv: GameState,
    args: Path<(StationId, ShipId, ShipModuleId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, ship_id, mod_id) = *args;

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                player
                    .buy_ship_module_upgrade(&station_id, &ship_id, &mod_id)
                    .await
                    .map(|(c, r)| json!({ "new-rank": r, "cost": c }))
            })
        })
        .await;
    build_response(data)
}

// @summary Buy a storage expansion for the station
// @returns The cargo of the station
#[web::post("/cargo/buy/{amount}")]
async fn buy_station_cargo(
    srv: GameState,
    args: Path<(StationId, usize)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, amnt) = *args;

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                player
                    .buy_station_cargo(&station_id, amnt)
                    .await
                    .map(|v| to_value(v).unwrap())
            })
        })
        .await;
    build_response(data)
}

pub fn configure<T: IntoPattern>(base: T, srv: &mut ServiceConfig) {
    srv.service(
        scope(base)
            .service(buy_ship_module)
            .service(get_ship_module_upgrade_prices)
            .service(buy_ship_module_upgrade)
            .service(get_prices_ship_module)
            .service(buy_station_cargo),
    );
}
