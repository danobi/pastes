use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Parser;
use log::{info, LevelFilter};
use rusqlite::{Connection, NO_PARAMS};
use simple_logger::SimpleLogger;
use tide::{Request, StatusCode};

#[derive(Parser, Debug)]
struct Args {
    #[structopt(short, long, default_value = "3400")]
    port: u16,
    #[structopt(long, default_value = "/etc/pastes.sqlite3", parse(from_os_str))]
    db: PathBuf,
}

// Shared application state
#[derive(Clone)]
struct State {
    db: PathBuf,
}

impl State {
    fn new(args: &Args) -> Result<Self> {
        Ok(Self {
            db: args.db.clone(),
        })
    }
}

fn get_connection(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "create table if not exists pastes (
             id string primary key unique,
             contents text not null
         )",
        NO_PARAMS,
    )?;

    Ok(conn)
}

async fn homepage(mut _req: Request<State>) -> tide::Result {
    let help = include_str!("./help.txt");
    Ok(help.into())
}

async fn get(req: Request<State>) -> tide::Result {
    let id = req.param("id")?;
    info!("get id={}", id);

    Ok(StatusCode::Ok.into())
}

async fn post(mut req: Request<State>) -> tide::Result {
    let body = req.body_string().await?;
    info!("post: {} bytes", body.len());

    Ok(StatusCode::Created.into())
}

#[async_std::main]
async fn main() -> tide::Result<()> {
    let args = Args::parse();
    let port = args.port;

    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let mut app = tide::with_state(State::new(&args)?);
    app.at("/").get(homepage);
    app.at("/:id").get(get);
    app.at("/").post(post);
    app.listen(format!("0.0.0.0:{port}")).await?;

    Ok(())
}
