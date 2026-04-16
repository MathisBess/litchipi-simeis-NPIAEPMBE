use ntex::router::IntoPattern;
use ntex::web;
use ntex::web::scope;
use ntex::web::types::Path;
use ntex::web::HttpRequest;
use ntex::web::ServiceConfig;

use serde_json::json;
use simeis_data::player::PlayerId;

use simeis_data::errors::Errcode;

use crate::api::build_response;
use crate::api::GameState;

// @summary Creates a new player in the game
// @returns The ID of the player, and its authentication secret key
// The secret key must be used in the "Simeis-Key" HTTP header
#[web::post("/new/{name}")]
async fn new_player(srv: GameState, name: Path<String>) -> impl web::Responder {
    let name = name.to_string();
    let res = srv.new_player(name).await.map(|(id, key)| {
        json!({
            "playerId": id,
            "key": key,
        })
    });
    build_response(res)
}

// @summary Get the status from the player of a given id.
// @returns Information about the player, complete if it's you, minimal if it's another player
#[web::get("/{player_id}")]
async fn get_player(srv: GameState, id: Path<PlayerId>, req: HttpRequest) -> impl web::Responder {
    let pkey = get_player_key!(req);
    let id = id.as_ref();
    let data = srv.player_to_json(&pkey, id).await;
    build_response(data)
}

pub fn configure<T: IntoPattern>(base: T, srv: &mut ServiceConfig) {
    srv.service(scope(base).service(get_player).service(new_player));
}
