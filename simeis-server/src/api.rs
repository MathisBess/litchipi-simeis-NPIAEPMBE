use ntex::web::{HttpResponse, ServiceConfig};
use serde_json::{json, Value};
use simeis_data::errors::Errcode;

use crate::GameState;

pub type ApiResult = Result<Value, Errcode>;

// TODO (#35) Pass complex requests via POST body instead of url
// - Navigation
// - Upgrades

macro_rules! get_player_key {
    ($req:ident) => {
        'getk: {
            use base64::{prelude::BASE64_STANDARD, Engine};
            let Some(b64key) = $req.headers().get("Simeis-Key") else {
                return build_response(Err(Errcode::NoPlayerKey));
            };
            let mut key = [0; 128];
            if !BASE64_STANDARD
                .decode_slice(b64key, &mut key)
                .ok()
                .is_some()
            {
                return build_response(Err(Errcode::InvalidPlayerKey));
            };
            break 'getk key;
        }
    };
}

#[inline]
pub fn jsonmerge(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for (k, v) in b {
                jsonmerge(a.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (a, b) => *a = b.clone(),
    }
}

#[inline]
fn build_response(res: ApiResult) -> HttpResponse {
    let body = match res {
        Ok(mut data) => {
            jsonmerge(&mut data, &json!({"error": "ok"}));
            data
        }
        Err(e) => {
            json!({"error": e.errmsg(), "type": format!("{e:?}")})
        }
    };

    HttpResponse::Ok()
        .content_type("application/json")
        .json(&body)
}

mod market;
mod player;
mod ship;
mod station;
mod system;

// Nested
mod crew;
mod industry;
mod shipyard;
mod station_shop;

// TODO (#35) Requires POST body implementation first
// TODO (#36) Endpoints for all kinds of information
// - Mining rate (+ wages cost) of a module, with a certain rank, with a certain operator rank, on a certain planet
// - Traveling costs to a particular destination, from a particular source
// - Prices of the upgrade of a module + its operator (from a source level to dest level), returns benefits, price, wages, etc...
// - Prices of the upgrade of an industry + its operator (from source lvl to dst lvl), returns benefits, price, wages, etc...
// - Resource requirements / production of an industry at a certain level
// - Price, wage, benefits on ships stats when upgrading a pilot
// - Price, wage, fee reduction when upgrading a trader
// TODO (#37) Document greatly the API

pub fn configure(srv: &mut ServiceConfig) {
    system::configure(srv);
    market::configure("/market", srv);
    player::configure("/player", srv);
    ship::configure("/ship/{ship_id}", srv);
    station::configure("/station/{station_id}", srv);
}
