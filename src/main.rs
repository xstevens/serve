#[macro_use]
extern crate clap;
extern crate itertools;
#[macro_use]
extern crate rocket;
extern crate chrono;

use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use clap::{App, Arg};
use rocket::config::Config;
use rocket::data::{Data, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::fs::NamedFile;
use rocket::http::Header;
use rocket::log::LogLevel;
use rocket::response::content;
use rocket::tokio;
use tokio::fs::File;
use tokio::io::{self, AsyncWriteExt};

#[get("/ping")]
fn ping() -> &'static str {
    "OK\r\n"
}

#[get("/robots.txt")]
fn robots() -> &'static str {
    "User-agent: *\r\nDisallow: /\r\n"
}

#[get("/static/<path..>")]
async fn files(path: PathBuf) -> Option<NamedFile> {
    let path = Path::new("./static").join(path);
    NamedFile::open(&path).await.ok()
}

#[post("/upload/<path..>", data = "<data>", rank = 10)]
async fn upload(path: PathBuf, data: Data<'_>) -> std::io::Result<()> {
    let fpath = Path::new("./upload/").join(path);
    if let Some(dir) = fpath.parent() {
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
    }
    let mut file = File::create(&fpath).await?;

    data.open(1.mebibytes()).stream_to(&mut file).await?;

    Ok(())
}

#[post("/dump", data = "<data>", rank = 1)]
async fn dump(data: Data<'_>) -> io::Result<()> {
    let mut stdout = rocket::tokio::io::stdout();
    data.open(1.mebibytes()).stream_to(&mut stdout).await?;
    stdout.write(b"\n").await?;

    Ok(())
}

#[catch(404)]
fn not_found(_req: &rocket::Request) -> content::Html<String> {
    content::Html(
r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>404 Not Found</title>
</head>
<body align="center">
    <div align="center">
        <h1>404: Not Found</h1>
        <p>The requested resource could not be found.</p>
    </div>
</body>
</html>"#.to_owned(),
    )
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let args = App::new("serve")
        .version(crate_version!())
        .about("a static HTTP server")
        .arg(
            Arg::with_name("cert")
                .long("cert")
                .value_name("CERT")
                .help("path to TLS certificate")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("key")
                .long("key")
                .value_name("KEY")
                .help("path to TLS private key")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("port")
                .long("port")
                .value_name("PORT")
                .default_value("8000")
                .help("port number to listen on")
                .takes_value(true),
        )
        .get_matches();

    // configuration
    let port: u16 = args.value_of("port").unwrap().parse().unwrap_or(8000);
    let mut config = Config::figment()
        .merge(("port", port))
        .merge(("address", "0.0.0.0"))
        .merge(("log_level", LogLevel::Off))
        .merge(("cli_colors", false));
    if args.is_present("cert") && args.is_present("key") {
        config = config
            .merge(("tls.certs", args.value_of("cert").unwrap()))
            .merge(("tls.key", args.value_of("key").unwrap()));
    }

    // setup rocket with custom fairing for request logging
    rocket::custom(config)
        .attach(AdHoc::on_request("request_log", |req, _| {
            Box::pin(async move {
                let now: DateTime<Utc> = Utc::now();
                let remote_addr: String = match req.remote() {
                    Some(addr) => format!("{}", addr.ip()),
                    _ => "-".to_owned(),
                };

                let referrer: &str = match req.headers().get_one("Referer") {
                    Some(referer) => {
                        if referer.len() == 0 {
                            "-"
                        } else {
                            referer
                        }
                    }
                    _ => "-",
                };

                let user_agent: &str = match req.headers().get_one("User-Agent") {
                    Some(ua) => ua,
                    _ => "-",
                };

                let mut cookies = itertools::join(req.headers().get("Cookie"), ",");
                if cookies.len() == 0 {
                    cookies = "-".to_owned();
                }

                let authz: &str = match req.headers().get_one("Authorization") {
                    Some(authz_header) => {
                        if authz_header.len() == 0 {
                            "-"
                        } else {
                            authz_header
                        }
                    }
                    _ => "-",
                };

                println!(
                    "{} {} {} {} \"{}\" \"{}\" \"{}\" \"{}\"",
                    now.to_rfc3339_opts(SecondsFormat::Millis, true),
                    remote_addr,
                    req.method(),
                    req.uri(),
                    referrer,
                    user_agent,
                    cookies,
                    authz
                );
            })
        }))
        .attach(AdHoc::on_response("server_response_header", |_, resp| {
            Box::pin(async move {
                resp.set_header(Header::new("Server", "NeXTcube"));
            })
        }))
        .mount("/", routes![ping, robots, files, upload, dump])
        .register("/", catchers![not_found])
}
