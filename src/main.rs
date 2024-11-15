#[macro_use]
extern crate clap;
extern crate itertools;
#[macro_use]
extern crate rocket;
extern crate chrono;

use std::path::{Path, PathBuf};
use std::collections::HashMap;

use chrono::{DateTime, SecondsFormat, Utc};
use clap::{Arg, ArgGroup, Command};
use rocket::config::Config;
use rocket::data::{Data, ToByteUnit};
use rocket::fairing::AdHoc;
use rocket::fs::NamedFile;
use rocket::http::Header;
use rocket::log::LogLevel;
use rocket::response::content;
use rocket::tokio;
use serde_json::json;
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
fn not_found(_req: &rocket::Request) -> content::RawHtml<String> {
    content::RawHtml(
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
    let cmd = Command::new("serve")
        .version(crate_version!())
        .about("a static HTTP server")
        .group(ArgGroup::new("tls")
            .args(["cert", "key"])
            .multiple(true)
        )
        .arg(
            Arg::new("cert")
                .long("cert")
                .value_name("CERT")
                .help("path to TLS certificate")
                .group("tls")
                .requires("key"),
        )
        .arg(
            Arg::new("key")
                .long("key")
                .value_name("KEY")
                .help("path to TLS private key")
                .group("tls")
                .requires("cert"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .value_name("PORT")
                .default_value("8000")
                .help("port number to listen on")
                .value_parser(clap::value_parser!(u16).range(8000..)),
        );
    let matches = cmd.get_matches();

    // configuration
    let port: u16 = *matches.get_one::<u16>("port").unwrap();
    let mut config = Config::figment()
        .merge(("port", port))
        .merge(("address", "0.0.0.0"))
        .merge(("log_level", LogLevel::Off))
        .merge(("cli_colors", false));

    if matches.contains_id("tls") {
        config = config
            .merge(("tls.certs", matches.get_one::<String>("cert").unwrap()))
            .merge(("tls.key", matches.get_one::<String>("key").unwrap()));
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

                // unfortunate but rocket HeaderMap is not serializable so copy
                // the name value pairs to a HashMap and pass that to the JSON
                // macro below
                let mut headers = HashMap::new();
                for h in req.headers().iter() {
                    headers.insert(h.name.as_str().to_string(), h.value().to_string());
                }

                let j = json!({
                    "ts": now.to_rfc3339_opts(SecondsFormat::Millis, true),
                    "remote_addr": remote_addr,
                    "method": req.method().to_string(),
                    "uri": req.uri(),
                    "headers": &headers,
                });
                println!("{}", j.to_string());
            })
        }))
        .attach(AdHoc::on_response("server_response_header", |_, resp| {
            Box::pin(async move {
                resp.set_header(Header::new("Server", "NeXTcube"));
                resp.set_header(Header::new("Accept-CH", "Sec-CH-UA-Full-Version,Sec-CH-UA-Platform,Sec-CH-UA-Platform-Version,Sec-CH-UA-Arch,Sec-CH-UA-Bitness,Sec-CH-UA-Model"));
            })
        }))
        .mount("/", routes![ping, robots, files, upload, dump])
        .register("/", catchers![not_found])
}
