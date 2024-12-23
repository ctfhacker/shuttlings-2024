use axum::{
    extract::{multipart::Multipart, Path},
    http::StatusCode,
    response::Html,
};
use serde::Deserialize;
use std::fmt::Display;
use std::str::FromStr;
use v_htmlescape::escape;

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Color {
    #[serde(alias = "red")]
    Red,
    #[serde(alias = "blue")]
    Blue,
    #[serde(alias = "purple")]
    Purple,
}

impl Color {
    pub fn next(self) -> Self {
        match self {
            Color::Red => Color::Blue,
            Color::Blue => Color::Purple,
            Color::Purple => Color::Red,
        }
    }
}

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = match self {
            Color::Red => "red",
            Color::Blue => "blue",
            Color::Purple => "purple",
        };
        write!(f, "{color}")
    }
}

impl FromStr for Color {
    type Err = StatusCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "red" => Ok(Color::Red),
            "blue" => Ok(Color::Blue),
            "purple" => Ok(Color::Purple),
            _ => Err(StatusCode::IM_A_TEAPOT),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
pub enum State {
    On,
    Off,
}

impl State {
    pub fn next(self) -> Self {
        match self {
            Self::On => Self::Off,
            Self::Off => Self::On,
        }
    }
}

impl FromStr for State {
    type Err = StatusCode;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "on" => Ok(State::On),
            "off" => Ok(State::Off),
            _ => Err(StatusCode::IM_A_TEAPOT),
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let color = match self {
            State::On => "on",
            State::Off => "off",
        };
        write!(f, "{color}")
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Lockfile {
    version: Option<u32>,
    package: Vec<Package>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Package {
    name: Option<String>,
    version: Option<String>,
    source: Option<String>,
    checksum: Option<String>,
    dependencies: Option<Vec<String>>,
}

pub async fn star() -> Html<String> {
    Html(r#"<div id="star" class="lit">"#.to_string())
}

pub async fn present(Path(color): Path<String>) -> Result<Html<String>, StatusCode> {
    let color = Color::from_str(&color)?;

    let next_color = color.next();

    let div = format!(
        r#"
    <div class="present {color}" hx-get="/23/present/{next_color}" hx-swap="outerHTML">
        <div class="ribbon"></div>
        <div class="ribbon"></div>
        <div class="ribbon"></div>
        <div class="ribbon"></div>
    </div>"#
    );

    Ok(Html(div.to_string()))
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrnamentParams {
    state: String,
    n: String,
}

pub async fn ornament(Path(params): Path<OrnamentParams>) -> Result<Html<String>, StatusCode> {
    let n = escape(&params.n);

    let state = State::from_str(&params.state)?;
    let next_state = state.next();
    let on = if state == State::On { " on" } else { "" };

    let div = format!(
        r#"
        <div 
            class="ornament{on}" 
            id="ornament{n}" 
            hx-trigger="load delay:2s once" 
            hx-get="/23/ornament/{next_state}/{n}" 
            hx-swap="outerHTML"
        >
        </div>
        "#
    );

    Ok(Html(div.to_string()))
}

pub async fn lockfile(mut multipart: Multipart) -> Result<String, (StatusCode, String)> {
    let mut divs = Vec::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid form".to_string()))?
    {
        if !matches!(field.name(), Some("lockfile")) {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Expected lockfile, found {:?}", field.name()),
            ));
        }

        // Parse the field bytes as a string
        let bytes = field.bytes().await.unwrap();
        let data =
            std::str::from_utf8(&bytes).map_err(|e| (StatusCode::IM_A_TEAPOT, format!("{e:?}")))?;

        // Parse the lockfile
        let lockfile: Lockfile =
            toml::from_str(data).map_err(|e| (StatusCode::BAD_REQUEST, format!("{e:?}")))?;

        // Parse each package's checksum
        for package in lockfile.package {
            let Some(checksum) = package.checksum else {
                continue;
            };

            macro_rules! parse_u8 {
                ($range:expr) => {{
                    let r = checksum.get($range).ok_or((
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Invalid checksum".to_string(),
                    ))?;

                    u8::from_str_radix(r, 16)
                        .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, format!("{e:?}")))
                }};
            }

            let r = parse_u8!(0..2)?;
            let g = parse_u8!(2..4)?;
            let b = parse_u8!(4..6)?;
            let top = parse_u8!(6..8)?;
            let left = parse_u8!(8..10)?;

            let div = format!(
                r#"
                <div style="background-color:#{r:02x}{g:02x}{b:02x};top:{top}px;left:{left}px;"></div>
            "#
            );

            divs.push(div);
        }
    }

    Ok(divs.join("\n"))
}
