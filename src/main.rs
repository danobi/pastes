use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use log::{error, info, LevelFilter};
use rand::Rng;
use rusqlite::{Connection, OptionalExtension};
use simple_logger::SimpleLogger;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;
use tide::{utils::After, Request, Response, StatusCode};

const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
const MAX_PASTE_SIZE: usize = 1 << 20;

#[derive(Parser, Debug)]
struct Args {
    #[structopt(long, default_value = "localhost")]
    addr: String,
    #[structopt(short, long, default_value = "3400")]
    port: u16,
    #[structopt(long, default_value = "./pastes.sqlite3", parse(from_os_str))]
    db: PathBuf,
}

// Shared application state
#[derive(Clone)]
struct State {
    db: PathBuf,
}

impl State {
    fn new(args: &Args) -> Self {
        Self {
            db: args.db.clone(),
        }
    }
}

fn get_connection(state: &State) -> Result<Connection> {
    let conn = Connection::open(&state.db)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pastes (
             id STRING PRIMARY KEY UNIQUE,
             contents TEXT NOT NULL
         )",
        [],
    )?;

    Ok(conn)
}

/// Generates a random id
fn gen_id() -> String {
    let mut rng = rand::thread_rng();

    (0..6)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Simple check to see if request is a browser. Not perfect, but good enough.
fn is_browser(req: &Request<State>) -> bool {
    if let Some(ua) = req.header("User-Agent") {
        ua.iter().any(|ua| {
            let s = ua.as_str();
            s.contains("Firefox")
                || s.contains("Chrome")
                || s.contains("Chromium")
                || s.contains("Safari")
        })
    } else {
        false
    }
}

/// Attempt to highlight the contents
fn highlight(c: &str) -> Option<String> {
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["InspiredGitHub"];
    let ss = SyntaxSet::load_defaults_newlines();
    let syntax = match ss.find_syntax_by_first_line(c) {
        Some(s) => s,
        None => {
            info!("Failed to determine syntax");
            return None;
        }
    };

    match highlighted_html_for_string(c, &ss, syntax, theme) {
        Ok(h) => Some(h),
        Err(e) => {
            error!("Error highlighting: {}", e);
            None
        }
    }
}

/// Response to a GET for a paste
fn respond_get(req: &Request<State>, c: &str) -> Response {
    if is_browser(req) {
        match highlight(c) {
            Some(h) => Response::builder(StatusCode::Ok)
                .content_type("text/html")
                .body(h)
                .build(),
            None => c.into(),
        }
    } else {
        c.into()
    }
}

async fn homepage(mut _req: Request<State>) -> tide::Result {
    let help = include_str!("./help.txt");
    Ok(help.into())
}

async fn get(req: Request<State>) -> tide::Result {
    let id = req.param("id")?;
    info!("GET id={}", id);

    let conn = get_connection(req.state())?;
    let contents: Option<String> = conn
        .query_row(
            r#"SELECT contents FROM pastes WHERE id=(?1)"#,
            [id],
            |row| row.get(0),
        )
        .optional()?;

    match contents {
        Some(c) => Ok(respond_get(&req, &c)),
        None => Ok(StatusCode::NotFound.into()),
    }
}

async fn post(mut req: Request<State>) -> tide::Result {
    let body = req.body_string().await?;
    info!("POST: {} bytes", body.len());
    if body.len() > MAX_PASTE_SIZE {
        return Ok(StatusCode::PayloadTooLarge.into());
    }

    let id = gen_id();
    let conn = get_connection(req.state())?;
    conn.execute(
        "INSERT INTO pastes (id, contents) VALUES (?1, ?2)",
        [&id, &body],
    )?;

    let mut resp = Response::new(StatusCode::Created);
    resp.set_body(format!("https://pastes.dxuuu.xyz/{id}\n"));

    Ok(resp)
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = Args::parse();
    let addr = args.addr.clone();
    let port = args.port;

    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let mut app = tide::with_state(State::new(&args));
    app.with(After(|res: Response| async {
        if let Some(err) = res.error() {
            error!("Internal error: {err}");
        }
        Ok(res)
    }));
    app.at("/").get(homepage);
    app.at("/:id").get(get);
    app.at("/").post(post);
    app.listen(format!("{addr}:{port}")).await?;

    Ok(())
}
