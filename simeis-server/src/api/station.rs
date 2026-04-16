use ntex::router::IntoPattern;
use ntex::web;
use ntex::web::scope;
use ntex::web::types::Path;
use ntex::web::HttpRequest;
use ntex::web::ServiceConfig;

use serde_json::json;
use serde_json::to_value;

use simeis_data::errors::Errcode;
use simeis_data::galaxy::station::StationId;
use simeis_data::ship::ShipId;

use crate::api::build_response;
use crate::api::GameState;

// @summary Get status of a station
// @returns All the informations for the player on this station
#[web::get("")]
async fn get_station_status(
    srv: GameState,
    id: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let id = id.as_ref();
    let key = get_player_key!(req);
    let data = srv
        .map_station(&key, id, |pid, station| {
            Box::pin(async { Ok(station.to_json(pid).await) })
        })
        .await;
    build_response(data)
}

// @summary Scan for planets around the station
// @returns Scan information on all the stellar objects around this station
#[web::post("/scan")]
async fn scan(id: Path<StationId>, srv: GameState, req: HttpRequest) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let station_id = *id;

    let data = srv
        .scan_galaxy(&pkey, &station_id)
        .await
        .map(|v| to_value(v).unwrap());
    build_response(data)
}

// @summary List the upgrades for a station currently available
// @returns List of the upgrades available, and their price
#[web::get("/upgrades")]
async fn get_station_upgrades(
    srv: GameState,
    args: Path<StationId>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let station_id = *args;

    let data = srv
        .map_station(&pkey, &station_id, |pid, station| {
            Box::pin(async move {
                let cargoprice = station.cargo_price(pid).await;
                let traderprice = station.upgr_trader_price(pid).await;
                Ok(json!({
                    "cargo": cargoprice,
                    "trader": traderprice,
                }))
            })
        })
        .await;
    build_response(data)
}

// @summary Use fuel in storage on the station to refuel the ship
// @returns How much fuel was effectively added to the ship (and removed from the station cargo)
#[web::post("/refuel/{ship_id}")]
async fn refuel_ship(
    srv: GameState,
    args: Path<(StationId, ShipId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, ship_id) = *args;

    let data = srv
        .map_ship_mut_in_station(&pkey, &station_id, &ship_id, |_, station, ship| {
            Box::pin(async move {
                station
                    .refuel_ship(ship)
                    .await
                    .map(|v| json!({ "added-fuel": v }))
            })
        })
        .await;
    build_response(data)
}

// @summary Use the hull plates in storage on the station to repair the ship
// @returns How much hull was effectively used to repair the ship (and removed from the station cargo)
#[web::post("/repair/{ship_id}")]
async fn repair_ship(
    srv: GameState,
    args: Path<(StationId, ShipId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let (station_id, ship_id) = *args;
    let data = srv
        .map_ship_mut_in_station(&pkey, &station_id, &ship_id, |_, station, ship| {
            Box::pin(async move {
                station
                    .repair_ship(ship)
                    .await
                    .map(|v| json!({ "added-hull": v }))
            })
        })
        .await;
    build_response(data)
}

pub fn configure<T: IntoPattern>(base: T, srv: &mut ServiceConfig) {
    srv.service(
        scope(base)
            .configure(|srv| crate::api::shipyard::configure("/shipyard", srv))
            .configure(|srv| crate::api::crew::configure("/crew", srv))
            .configure(|srv| crate::api::station_shop::configure("/shop", srv))
            .configure(|srv| crate::api::industry::configure("/industry", srv))
            .service(scan)
            .service(get_station_status)
            .service(get_station_upgrades)
            .service(refuel_ship)
            .service(repair_ship),
    );
}
