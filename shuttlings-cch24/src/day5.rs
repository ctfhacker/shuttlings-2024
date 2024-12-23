#![allow(dead_code)]

use axum::{
    extract::{Path, State},
    http::StatusCode,
};
use bon::bon;
use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Default, Copy, Clone, PartialEq)]
pub enum Piece {
    #[default]
    Empty,
    Wall,
    Cookie,
    Milk,
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub enum Team {
    #[serde(alias = "cookie")]
    Cookie,
    #[serde(alias = "milk")]
    Milk,
}

impl From<Team> for Piece {
    fn from(val: Team) -> Self {
        match val {
            Team::Cookie => Self::Cookie,
            Team::Milk => Self::Milk,
        }
    }
}

impl std::fmt::Display for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let piece = match self {
            Piece::Wall => '⬜',
            Piece::Cookie => '🍪',
            Piece::Milk => '🥛',
            Piece::Empty => '⬛',
        };

        write!(f, "{piece}")
    }
}

impl std::fmt::Debug for Piece {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{self}")
    }
}

const WIDTH: usize = 6;
const HEIGHT: usize = 5;

#[derive(Debug)]
pub struct Board {
    grid: [Piece; WIDTH * HEIGHT],
    winner: Option<Piece>,
    finished: bool,
    rng: StdRng,
}

impl Default for Board {
    fn default() -> Board {
        Board::new()
    }
}

#[bon]
impl Board {
    #[builder]
    pub fn has_piece(&mut self, row: usize, col: usize) -> bool {
        matches!(self.grid[row * WIDTH + col], Piece::Cookie | Piece::Milk)
    }

    #[builder]
    pub fn set_piece(&mut self, row: usize, col: usize, piece: Piece) {
        self.grid[row * WIDTH + col] = piece;
        self.check_finished();
    }

    #[builder]
    pub fn get_piece(&self, row: usize, col: usize) -> Piece {
        self.grid[row * WIDTH + col]
    }

    pub fn random_board(&mut self) {
        // Reset winner
        self.winner = None;

        // Fill the vertical sides of the board
        for row in 0..(HEIGHT - 1) {
            // Fill the bottom edge of the board
            for col in 1..(WIDTH - 1) {
                let piece = if self.rng.gen::<bool>() {
                    Piece::Cookie
                } else {
                    Piece::Milk
                };

                self.set_piece().row(row).col(col).piece(piece).call();
            }
        }

        self.check_winner();
    }

    /// Check if the board is finished
    fn check_finished(&mut self) {
        self.finished = !self.grid.iter().any(|x| *x == Piece::Empty);
    }

    #[builder]
    pub fn play_piece(&mut self, team: &str, col: usize) -> Result<(), (StatusCode, String)> {
        if self.finished {
            return Err((StatusCode::SERVICE_UNAVAILABLE, format!("{self}")));
        }

        if !(1..=4).contains(&col) {
            return Err((StatusCode::BAD_REQUEST, String::new()));
        }

        let team = match team {
            "cookie" => Piece::Cookie,
            "milk" => Piece::Milk,
            _ => {
                return Err((StatusCode::BAD_REQUEST, String::new()));
            }
        };

        for row in (0..4).rev() {
            if self.has_piece().row(row).col(col).call() {
                continue;
            }

            self.set_piece().row(row).col(col).piece(team).call();

            return Ok(());
        }

        Err((StatusCode::SERVICE_UNAVAILABLE, format!("{self}")))
    }

    pub fn reset(&mut self) {
        *self = Board::new();
    }

    pub fn check_winner(&mut self) {
        if self.winner.is_some() {
            return;
        }

        // The valid positions for a connect 4
        let coords = [
            // Rows
            [(0, 1), (0, 2), (0, 3), (0, 4)],
            [(1, 1), (1, 2), (1, 3), (1, 4)],
            [(2, 1), (2, 2), (2, 3), (2, 4)],
            [(3, 1), (3, 2), (3, 3), (3, 4)],
            // Columns
            [(0, 1), (1, 1), (2, 1), (3, 1)],
            [(0, 2), (1, 2), (2, 2), (3, 2)],
            [(0, 3), (1, 3), (2, 3), (3, 3)],
            [(0, 4), (1, 4), (2, 4), (3, 4)],
            // Diagonals
            [(0, 1), (1, 2), (2, 3), (3, 4)],
            [(3, 1), (2, 2), (1, 3), (0, 4)],
        ];

        for coord in coords {
            let mut pieces = [Piece::Empty; 4];

            for (i, (row, col)) in coord.iter().enumerate() {
                pieces[i] = self.get_piece().row(*row).col(*col).call();
            }

            if pieces == [Piece::Milk; 4] || pieces == [Piece::Cookie; 4] {
                self.winner = Some(pieces[0]);
                self.finished = true;
                break;
            }
        }
    }

    pub fn new() -> Board {
        let mut board = Board {
            grid: [Piece::default(); WIDTH * HEIGHT],
            winner: None,
            finished: false,
            rng: StdRng::seed_from_u64(2024),
        };

        // Fill the vertical sides of the board
        for row in 0..HEIGHT {
            board.set_piece().row(row).col(0).piece(Piece::Wall).call();

            board
                .set_piece()
                .row(row)
                .col(WIDTH - 1)
                .piece(Piece::Wall)
                .call();
        }

        // Fill the bottom edge of the board
        for col in 0..WIDTH {
            board
                .set_piece()
                .row(HEIGHT - 1)
                .col(col)
                .piece(Piece::Wall)
                .call();
        }

        board
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                write!(f, "{}", self.get_piece().row(row).col(col).call())?;
            }

            writeln!(f)?;
        }

        if let Some(winner) = self.winner {
            writeln!(f, "{winner:?} wins!")?;
        } else if self.finished {
            writeln!(f, "No winner.")?;
        }

        Ok(())
    }
}

pub async fn board(board: State<Arc<Mutex<Board>>>) -> String {
    format!("{}", board.lock().unwrap())
}

pub async fn reset_board(board: State<Arc<Mutex<Board>>>) -> String {
    let mut board = board.lock().unwrap();
    board.reset();
    format!("{board}")
}
#[derive(Deserialize)]
pub struct PlacePieceParams {
    team: Team,
    column: usize,
}

pub async fn place_piece(
    board: State<Arc<Mutex<Board>>>,
    Path((team, column)): Path<(String, usize)>,
) -> Result<String, (StatusCode, String)> {
    let mut board = board.lock().unwrap();
    board.play_piece().team(&team).col(column).call()?;
    board.check_winner();
    Ok(format!("{board}"))
}

pub async fn random_board(
    board: State<Arc<Mutex<Board>>>,
) -> Result<String, (StatusCode, &'static str)> {
    let mut board = board.lock().unwrap();
    board.random_board();
    Ok(format!("{board}"))
}

#[cfg(test)]
mod day5_tests {
    use crate::app;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn get_board() {
        let app = app();

        let response = app
            .clone()
            .oneshot(Request::get("/12/board").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();
        assert_eq!(
            body,
            "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
        );
    }

    #[tokio::test]
    async fn get_reset() {
        let app = app();

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/reset".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
        );
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn get_place_piece() {
        let app = app();

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/place/cookie/1".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
        );

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/place/cookie/1".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜⬛⬛⬛⬛⬜
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
        );

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/place/cookie/1".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜⬛⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
"
        );

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/place/cookie/1".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜🍪⬛⬛⬛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
        );

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/place/cookie/1".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_place_pieces2() {
        let app = app();

        let moves = [
            "/12/reset",
            "/12/place/cookie/1",
            "/12/place/milk/2",
            "/12/place/cookie/2",
            "/12/place/milk/3",
            "/12/place/milk/3",
            "/12/place/cookie/3",
            "/12/place/milk/4",
            "/12/place/milk/4",
            "/12/place/milk/4",
        ];

        for next_move in moves {
            let response = app
                .clone()
                .oneshot(
                    Request::post(next_move.to_string())
                        .body(Body::default())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);
        }

        let last_move = "/12/place/cookie/4";
        let response = app
            .clone()
            .oneshot(
                Request::post(last_move.to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜⬛⬛⬛🍪⬜
⬜⬛⬛🍪🥛⬜
⬜⬛🍪🥛🥛⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
        );
    }

    #[tokio::test]
    async fn test_random_board() {
        let app = app();

        let response = app
            .clone()
            .oneshot(
                Request::post("/12/reset".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = app
            .clone()
            .oneshot(
                Request::get("/12/random-board".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜🍪🍪🍪🍪⬜
⬜🥛🍪🍪🥛⬜
⬜🥛🥛🥛🥛⬜
⬜🍪🥛🍪🥛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
"
        );
        let response = app
            .clone()
            .oneshot(
                Request::get("/12/random-board".to_string())
                    .body(Body::default())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body: String = std::str::from_utf8(&body).unwrap().chars().collect();

        assert_eq!(
            body,
            "\
⬜🍪🥛🍪🍪⬜
⬜🥛🍪🥛🍪⬜
⬜🥛🍪🍪🍪⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
No winner.
"
        );
    }
}

/*
curl http://localhost:8000/12/random-board
⬜🍪🥛🍪🍪⬜
⬜🥛🍪🥛🍪⬜
⬜🥛🍪🍪🍪⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
No winner.
*/

/*
curl -X POST http://localhost:8000/12/reset
curl -X POST http://localhost:8000/12/place/cookie/1
curl -X POST http://localhost:8000/12/place/milk/2
curl -X POST http://localhost:8000/12/place/cookie/2
curl -X POST http://localhost:8000/12/place/milk/3
curl -X POST http://localhost:8000/12/place/milk/3
curl -X POST http://localhost:8000/12/place/cookie/3
curl -X POST http://localhost:8000/12/place/milk/4
curl -X POST http://localhost:8000/12/place/milk/4
curl -X POST http://localhost:8000/12/place/milk/4
# The output from the above calls is hidden for brevity

curl -X POST http://localhost:8000/12/place/cookie/4
⬜⬛⬛⬛🍪⬜
⬜⬛⬛🍪🥛⬜
⬜⬛🍪🥛🥛⬜
⬜🍪🥛🥛🥛⬜
⬜⬜⬜⬜⬜⬜
🍪 wins!
*/
