#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate clap;
extern crate itertools;
#[macro_use] extern crate rocket;
extern crate time;

use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{App, Arg};
use rocket::Data;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::response::NamedFile;
use rocket::response::content;

#[get("/ping")]
fn ping() -> &'static str {
    "OK\r\n"
}

#[get("/static/<path..>")]
fn files(path: PathBuf) -> Option<NamedFile> {
    let path = Path::new("./static").join(path);
    NamedFile::open(&path).ok()
}

#[post("/upload/<path..>", data = "<data>", rank = 10)]
fn upload(path: PathBuf, data: Data) -> io::Result<()> {
    let path = Path::new("./upload/").join(path);
    if !path.exists() {
        fs::create_dir_all(&path)?;
    }
    data.stream_to_file(&path)?;
    Ok(())
}

#[post("/", data = "<data>", rank = 1)]
fn dump(data: Data) -> io::Result<()> {
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    data.stream_to(&mut writer)?;
    writer.write(b"\n")?;

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
</html>"#.to_owned()
    )
}

fn main() {
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
        .get_matches();
    // configuration
    let mut config = Config::new(Environment::Production);
    config.set_address("0.0.0.0").unwrap();
    config.set_port(8000);
    config.set_log_level(LoggingLevel::Off);
    if args.is_present("cert") && args.is_present("key") {
        config.set_tls(args.value_of("cert").unwrap(), args.value_of("key").unwrap()).unwrap();
    }
    // setup rocket with custom fairing for request logging
    rocket::custom(config)
        .attach(AdHoc::on_request("request_log", |req, _| {
            let ts = time::strftime("%Y-%m-%dT%H:%M:%S.%fZ", &time::now_utc()).unwrap();

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
                },
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
                },
                _ => "-",
            };

            println!("{} {} {} {} \"{}\" \"{}\" \"{}\" \"{}\"", ts, remote_addr, req.method(), req.uri(), referrer, user_agent, cookies, authz);
        }))
        .attach(AdHoc::on_response("server_response_header", |_, resp| {
            resp.set_header(Header::new("Server", "NeXTcube"));
        }))
        .mount("/", routes![ping, files, upload, dump])
        .register(catchers![not_found])
        .launch();
}
