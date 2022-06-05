use clap::Parser;
use log::{info, LevelFilter};
use simple_logger::SimpleLogger;
use tide::{Request, StatusCode};

#[derive(Parser, Debug)]
struct Args {
    #[structopt(short, long, default_value = "3400")]
    port: u16,
}

async fn homepage(mut _req: Request<()>) -> tide::Result {
    let help = include_str!("./help.txt");
    Ok(help.into())
}

async fn get(req: Request<()>) -> tide::Result {
    let id = req.param("id")?;
    info!("get id={}", id);

    Ok(StatusCode::Ok.into())
}

async fn post(mut req: Request<()>) -> tide::Result {
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

    let mut app = tide::new();
    app.at("/").get(homepage);
    app.at("/:id").get(get);
    app.at("/").post(post);
    app.listen(format!("0.0.0.0:{port}")).await?;

    Ok(())
}
