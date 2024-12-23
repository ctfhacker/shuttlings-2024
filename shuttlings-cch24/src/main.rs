#![deny(clippy::pedantic)]

use axum::{
    extract::FromRef,
    routing::{delete, get, post, put},
    Extension, Router,
};
use jsonwebtoken::DecodingKey;
use std::sync::{Arc, Mutex};
use tower_http::services::ServeDir;

mod day1;
mod day2;
mod day3;
mod day4;
mod day5;
use day5::Board;
mod day6;
mod day7;
mod day8;

#[derive(Clone)]
struct SantaState {
    board: Arc<Mutex<Board>>,
    pubkey: Arc<DecodingKey>,
}

impl FromRef<SantaState> for Arc<Mutex<Board>> {
    fn from_ref(state: &SantaState) -> Arc<Mutex<Board>> {
        state.board.clone()
    }
}

impl FromRef<SantaState> for Arc<DecodingKey> {
    fn from_ref(state: &SantaState) -> Arc<DecodingKey> {
        state.pubkey.clone()
    }
}

impl SantaState {
    pub fn new() -> Self {
        let pem = include_bytes!("../day16_santa_public_key.pem");
        let key = if let Ok(key) = DecodingKey::from_ec_pem(pem) {
            key
        } else if let Ok(key) = DecodingKey::from_ed_pem(pem) {
            key
        } else if let Ok(key) = DecodingKey::from_rsa_pem(pem) {
            key
        } else {
            panic!("Invalid public key from santa");
        };

        Self {
            board: Arc::new(Mutex::new(Board::new())),
            pubkey: Arc::new(key),
        }
    }
}

fn app() -> Router {
    let limiter = day4::create_milk_limiter();
    let limiter = Arc::new(Mutex::new(limiter));

    Router::new()
        .route("/-1/seek", get(day1::seek))
        .route("/2/dest", get(day2::ipv4_dest))
        .route("/2/key", get(day2::ipv4_key))
        .route("/2/v6/dest", get(day2::ipv6_dest))
        .route("/2/v6/key", get(day2::ipv6_key))
        .route("/5/manifest", post(day3::manifest))
        .route("/9/milk", post(day4::milk))
        .route("/9/refill", post(day4::refill))
        .route("/12/board", get(day5::board))
        .route("/12/reset", post(day5::reset_board))
        .route("/12/place/:team/:column", post(day5::place_piece))
        .route("/12/random-board", get(day5::random_board))
        .route("/16/wrap", post(day6::wrap))
        .route("/16/unwrap", get(day6::unwrap))
        .route("/16/decode", post(day6::decode))
        .route("/19/reset", post(day7::reset))
        .route("/19/draft", post(day7::draft))
        .route("/19/cite/:id", get(day7::cite))
        .route("/19/remove/:id", delete(day7::remove))
        .route("/19/undo/:id", put(day7::undo))
        .route("/19/list", get(day7::list))
        .route("/23/star", get(day8::star))
        .route("/23/present/:color", get(day8::present))
        .route("/23/ornament/:state/:n", get(day8::ornament))
        .route("/23/lockfile", post(day8::lockfile))
        .layer(Extension(limiter))
        .with_state(SantaState::new())
        .nest_service("/assets", ServeDir::new("assets"))
}

#[shuttle_runtime::main]
#[allow(clippy::unused_async)]
async fn main(#[shuttle_shared_db::Postgres] pool: sqlx::PgPool) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    Ok(app().layer(Extension(Arc::new(pool))).into())
}
