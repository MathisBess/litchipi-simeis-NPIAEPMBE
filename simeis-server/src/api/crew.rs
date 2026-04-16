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

use simeis_data::crew::CrewId;
use simeis_data::crew::CrewMemberType;
use simeis_data::errors::Errcode;
use simeis_data::galaxy::station::StationId;
use simeis_data::industry::IndustryUnitId;
use simeis_data::ship::module::ShipModuleId;
use simeis_data::ship::ShipId;

use crate::api::build_response;
use crate::api::GameState;

// @summary Hire a new crew member on the station. Unless assigned, it will stay idle
// @returns The ID of the hired crew member
#[web::post("/hire/{crewtype}")]
async fn hire_crew(
    srv: GameState,
    args: Path<(StationId, String)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crewtype) = args.as_ref();
    let Ok(crewtype) = CrewMemberType::from_str(crewtype.as_str()) else {
        return build_response(Err(Errcode::InvalidArgument("crewtype")));
    };

    let pkey = get_player_key!(req);
    let data = srv
        .map_station(&pkey, station_id, |pid, station| {
            Box::pin(async move {
                let id = station.hire_crew(pid, crewtype).await;
                Ok(json!({ "id": id}))
            })
        })
        .await;
    let _ = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async {
                player.update_costs().await;
                Ok(())
            })
        })
        .await;
    build_response(data)
}

// @summary List all the upgrades available for the crew of a specific ship
// @returns For each member, his type, rank, and price for next rank
#[web::get("/upgrade/ship/{ship_id}")]
async fn get_crew_upgrades(
    srv: GameState,
    args: Path<(StationId, ShipId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, ship_id) = *args;
    let pkey = get_player_key!(req);
    let data = srv
        .map_player(&pkey, |player| {
            Box::pin(async move {
                if !player.ship_in_station(&ship_id, &station_id)? {
                    return Err(Errcode::ShipNotInStation);
                }
                // SAFETY Checked on the ship_in_station function
                let ship = player.ships.get(&ship_id).unwrap();

                let mut res = BTreeMap::new();
                for (cid, cm) in ship.crew.0.iter() {
                    res.insert(
                        cid,
                        json!({
                            "member-type": cm.member_type,
                            "rank": cm.rank + 1,
                            "price": cm.price_next_rank(),
                        }),
                    );
                }
                Ok(to_value(res).unwrap())
            })
        })
        .await;
    build_response(data)
}

// @summary Upgrade a crew member of a specific ship
// @returns New rank, and cost of the upgrade
#[web::post("/upgrade/ship/{ship_id}/{crew_id}")]
async fn upgrade_ship_crew(
    srv: GameState,
    args: Path<(StationId, ShipId, CrewId)>,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, ship_id, crew_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                let res = player
                    .upgrade_ship_crew(&station_id, &ship_id, &crew_id)
                    .await;
                match res {
                    Ok((p, r)) => {
                        player.update_costs().await;
                        Ok(json!({ "new-rank": r, "cost": p }))
                    }
                    Err(e) => Err(e),
                }
            })
        })
        .await;
    build_response(data)
}

// @summary Upgrade a crew member of the station
// @returns New rank, and cost of the upgrade
#[web::post("/upgrade/{crew_id}")]
async fn upgrade_station_crew(
    args: Path<(StationId, CrewId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_player_mut(&pkey, |player| {
            Box::pin(async move {
                player
                    .upgrade_station_crew(&station_id, &crew_id)
                    .await
                    .map(|(p, r)| json!({ "new-rank": r, "cost": p }))
            })
        })
        .await;
    build_response(data)
}

// @summary Assign a crew member as a trader on a station.
// @returns Nothing
// The level of the trader will affect the fee rate applied on the market
#[web::post("/assign/{crew_id}/trading")]
async fn assign_trader(
    args: Path<(StationId, CrewId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_station(&pkey, &station_id, |pid, station| {
            Box::pin(async move {
                station.assign_trader(pid, crew_id).await?;
                Ok(json!({}))
            })
        })
        .await;
    build_response(data)
}

// @summary Assign a crew member as a pilot on a ship.
// @returns Nothing
// The level of the pilot will affect the speed of the ship, as well as it's fuel consumption
#[web::post("/assign/{crew_id}/ship/{ship_id}/pilot")]
async fn assign_pilot(
    args: Path<(StationId, CrewId, ShipId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id, ship_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_ship_mut_in_station(&pkey, &station_id, &ship_id, |_, station, ship| {
            Box::pin(async move {
                if ship.pilot.is_some() {
                    return Err(Errcode::CrewNotNeeded);
                }
                station
                    .onboard_pilot(ship, &crew_id)
                    .await
                    .map(|_| json!({}))
            })
        })
        .await;
    build_response(data)
}

// @summary Assign a crew member as an operator on a ship.
// @returns Nothing
// The level of the crew member will affect the extraction rate of the resources
#[web::post("/assign/{crew_id}/ship/{ship_id}/{mod_id}")]
async fn assign_operator_to_ship(
    args: Path<(StationId, CrewId, ShipId, ShipModuleId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id, ship_id, mod_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_ship_mut_in_station(&pkey, &station_id, &ship_id, |_, station, ship| {
            Box::pin(async move {
                station
                    .onboard_operator(ship, &crew_id, &mod_id)
                    .await
                    .map(|_| json!({}))
            })
        })
        .await;
    build_response(data)
}

// @summary Assign a crew member as an operator on an industry unit of a sttion.
// @returns Nothing
// The level of the crew member will affect the production rate
#[web::post("/assign/{crew_id}/industry/{industry_id}")]
async fn assign_operator_to_industry(
    args: Path<(StationId, CrewId, IndustryUnitId)>,
    srv: GameState,
    req: HttpRequest,
) -> impl web::Responder {
    let (station_id, crew_id, industry_id) = *args;
    let pkey = get_player_key!(req);

    let data = srv
        .map_station(&pkey, &station_id, |pid, station| {
            Box::pin(async move {
                station
                    .assign_crew_to_industry(pid, &crew_id, &industry_id)
                    .await
                    .map(|_| json!({}))
            })
        })
        .await;
    build_response(data)
}

pub fn configure<T: IntoPattern>(base: T, srv: &mut ServiceConfig) {
    srv.service(
        scope(base)
            .service(hire_crew)
            .service(get_crew_upgrades)
            .service(upgrade_ship_crew)
            .service(upgrade_station_crew)
            .service(assign_pilot)
            .service(assign_operator_to_industry)
            .service(assign_operator_to_ship)
            .service(assign_trader),
    );
}
