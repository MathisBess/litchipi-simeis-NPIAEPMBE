mod sdk;

use std::time::Duration;

use sdk::*;

pub struct Game {
    sdk: sdk::SimeisSDK,
}

impl Game {
    pub fn new(username: String) -> Game {
        Game {
            sdk: sdk::SimeisSDK::new(username, "0.0.0.0", 8080),
        }
    }

    pub fn gameloop(&self) -> Result<(), serde_json::Value> {
        let status = self.sdk.get_player_status()?;
        println!("{status:?}");
        println!("{:?}", json_get_key("stations", &status));
        println!("{:?}", json_get_list("stations", &status));
        let all_stations = json_get_list("stations", &status).unwrap();
        let station_id = all_stations.first().unwrap().as_u64().unwrap();

        // On a besoin de savoir quelle planète miner pour équiper notre vaisseau
        let all_planets = self.sdk.scan_planets(station_id)?;
        let nearest_planet = all_planets.first().unwrap();
        let nearest_planet_pos = get_position(nearest_planet).unwrap();
        println!("Targeting planet {nearest_planet:?}");

        // Si on commence une nouvelle partie, on s'équipe
        let all_my_ships = json_get_list("ships", &status).unwrap();
        let ship;
        let ship_id;
        if all_my_ships.is_empty() {
            println!("Buying first ship");
            let list_all_ships = self.sdk.list_shop_ship(station_id)?;
            ship = list_all_ships.first().unwrap();
            ship_id = get_id(ship);

            self.sdk.buy_ship(station_id, ship_id)?;

            // En fonction de la planète, on achète un module de minage différent
            let planet_is_solid = json_get_bool("solid", nearest_planet).unwrap();
            let module = if planet_is_solid {
                "Miner"
            } else {
                "GasSucker"
            };
            let module = self.sdk.buy_module_on_ship(station_id, ship_id, module)?;
            let mod_id = get_id(&module);

            // On embauche du personnel
            let operator = self.sdk.hire_crew(station_id, "operator")?;
            let operator_id = get_id(&operator);
            self.sdk
                .assign_crew_to_ship_module(station_id, ship_id, operator_id, mod_id)?;

            let pilot = self.sdk.hire_crew(station_id, "pilot")?;
            let pilot_id = get_id(&pilot);
            self.sdk
                .assign_crew_as_ship_pilot(station_id, ship_id, pilot_id)?;

            let trader = self.sdk.hire_crew(station_id, "trader")?;
            let trader_id = get_id(&trader);
            self.sdk.assign_trader_to_station(station_id, trader_id)?;
        }
        // Si on reprends une partie existante
        // On retourne à la station, on vide tout, avant de repartir
        else {
            ship = all_my_ships.first().unwrap();
            ship_id = get_id(ship);
            self.sdk.return_station_and_unload_all(station_id, ship_id)?;
        }

        // Cycle infini
        //     On va à la planète
        //     On mine
        //     On rentre à la station
        //     On répare le vaisseau, on fait le plein
        //     On vends les resources
        loop {
            let status = self.sdk.get_player_status()?;
            let money = json_get_float("money", &status).unwrap();
            let costs = json_get_float("costs", &status).unwrap();
            println!(
                "Current status: {:.2} credits, costs: {:.2}, time left before lost: {} secs",
                money,
                costs,
                (money / costs).round() as u32,
            );
            if money <= 0.0 {
                println!("You lost");
                break;
            }

            // On va à la planète
            self.sdk.travel(ship_id, nearest_planet_pos, true)?;

            // On mine
            let prices = self.sdk.get_market_prices()?;
            let info = self.sdk.start_extraction(ship_id)?;
            let stats = json_get_dict("mining_rate", &info).unwrap();
            let twait = json_get_float("time_fill_cargo", &info).unwrap();
            let mut totpersec = 0.0;
            for (res, amnt) in stats.iter() {
                let amnt = amnt.as_f64().unwrap();
                let price = prices.get(res.as_str()).unwrap();
                println!("{res}: {amnt} /sec");
                totpersec += amnt * price;
            }
            println!("Total: {totpersec} credits / sec");

            // On attends que l'extraction termine
            // Elle se termine automatiquement quand le cargo est plein
            std::thread::sleep(Duration::from_secs_f64(twait));
            self.sdk
                .wait_until_ship_idle(ship_id, Duration::from_millis(200))?;

            // On retourne à la station, et on décharge le cargo
            self.sdk.return_station_and_unload_all(station_id, ship_id)?;

            // On vends tout
            let mut cycletot = 0.0;
            let station_resources = self.sdk.get_station_resources(station_id)?;
            for (res, amnt) in station_resources.iter() {
                if ["Fuel", "Hull"].contains(&res.as_str()) {
                    continue;
                }

                let tx = self.sdk.sell_resource(station_id, res, *amnt)?;
                let added_money = json_get_float("added_money", &tx).unwrap();
                println!(
                    "Sold {amnt} of {res} for {added_money} credits (fees {} credits)",
                    json_get_float("fees", &tx).unwrap(),
                );
                cycletot += added_money;
            }

            // On achète du carburant et on fait le plein
            if let Some(tx) = self.sdk.buy_fuel_for_refuel(station_id, ship_id)? {
                let removed_money = json_get_float("removed_money", &tx).unwrap();
                cycletot -= removed_money;

                let added = json_get_list("added_cargo", &tx)
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_f64()
                    .unwrap();
                println!(
                    "Bought {added} of Fuel for {removed_money} credits (fees {} credits)",
                    json_get_float("fees", &tx).unwrap(),
                );
            }
            self.sdk.refuel_ship(station_id, ship_id)?;

            // On achète des plaques de coque, et on répare la coque
            if let Some(tx) = self.sdk.buy_hull_for_repair(station_id, ship_id)? {
                let removed_money = json_get_float("removed_money", &tx).unwrap();
                cycletot -= removed_money;
                let added = json_get_list("added_cargo", &tx)
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_f64()
                    .unwrap();
                println!(
                    "Bought {added} of Hull for {removed_money} credits (fees {} credits)",
                    json_get_float("fees", &tx).unwrap(),
                );
            }
            self.sdk.repair_ship(station_id, ship_id)?;

            // Rebelotte
            println!("Total this cycle: {cycletot}");
            println!();
        }
        Ok(())
    }
}

fn main() {
    let name = std::env::args()
        .nth(1)
        .expect("Requires the username as an argument");
    let game = Game::new(name);
    game.gameloop().expect("Uncaught error when calling API");
}
