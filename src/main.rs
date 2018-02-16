#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(use_extern_macros)]

extern crate itertools;
#[macro_use(log)]
extern crate log;
extern crate rocket;
extern crate time;

use std::io;
use std::path::{Path, PathBuf};

use rocket::Data;
use rocket::config::{Config, Environment};
use rocket::fairing::AdHoc;
use rocket::http::Header;
use rocket::response::NamedFile;

#[get("/ping")]
fn ping() -> &'static str {
    "OK\r\n"
}

#[get("/static/<path..>")]
fn files(path: PathBuf) -> Option<NamedFile> {
    let path = Path::new("./").join(path);
    NamedFile::open(&path).ok()
}

#[post("/upload/<path..>", data = "<data>")]
fn upload(path: PathBuf, data: Data) -> io::Result<()> {
    let path = Path::new("./upload/").join(path);
    data.stream_to_file(&path)?;
    Ok(())
}

fn main() {
    let config = Config::build(Environment::Production)
                        .address("0.0.0.0")
                        .port(8000)
                        .finalize()
                        .unwrap();

    rocket::custom(config, false)
        .attach(AdHoc::on_request(|req, _| {
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

            println!("{} {} {} {} \"{}\" \"{}\" \"{}\"", ts, remote_addr, req.method(), req.uri(), referrer, user_agent, cookies);
        }))
        .attach(AdHoc::on_response(|_, resp| {
            resp.set_header(Header::new("Server", "NeXTcube"));
        }))
        .mount("/", routes![ping, files, upload])
        .launch();
}
