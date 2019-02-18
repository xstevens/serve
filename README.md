# serve
An HTTP server intended to be useful for static serving and pen testing.

# What the hell is this?
During the course of a penetration test or CTF it can be useful to have an HTTP/HTTPS server that you can call out too. Frequently I find myself having a few needs from this:

- A server to call out to verify blind techniques
- A server to host static payloads
- A server to upload files to via HTTP

This is a playground project for me to do all of the above and learn some Rust at the same time.

# Why not just use...?
I have and probably will continue to use Python's SimpleHTTPServer, nginx, netcat, etc. but it is really handy to have a single executable that can do a bunch of these things with little to no configuration to manage.

# Is this just for penetration testing?
No. It doesn't have to be. You could host a statically generated site with this if you wanted. Probably needs a bit of hacking to get some paths to auto-forward, but it would probably do the job. Make sure you disable the upload route if you're going down this path.

# Build
Server is written in Rust so you can use cargo to build, install, etc.

```
cargo build --release
```

# Run
There are some issues at the moment due to using [rocket.rs](https://rocket.rs). One was in order to customize logging I instatiate using `rocket::custom`. This unfortunately disables `Rocket.toml` and environment configuration support. Customized logging is supposedly in the works, so we'll see what that looks like when it lands. Rocket's default logging choices were suboptimal for automated parsing.

**NOTE:** When you run `serve` it currently expects to have all files in a subdirectory called `static`. This is so you don't accidentally offer up files in other subdirectories.
```
./target/release/serve
```

# Example Runtime

## Client
```
$ curl http://localhost:8000/ping
OK
$ curl http://localhost:8000/static/src/main.rs
...
$ echo "Hello, world." | curl --data-binary @- http://localhost:8000/upload/test
```
## Server
```
$ target/release/serve
2018-01-14T19:07:57.143870000Z 127.0.0.1 GET /ping "-" "curl/7.54.0" "-"
2018-01-14T19:08:03.182927000Z 127.0.0.1 GET /static/src/main.rs "-" "curl/7.54.0" "-"
2018-01-14T19:08:07.815893000Z 127.0.0.1 POST /upload/test "-" "curl/7.54.0" "-"
^C
$ cat upload/test
Hello, world.
```

If you want to use TLS all you need to do is specify paths to a certificate and key using the command-line flags.

```
serve --cert ./certs/server.pem --key ./certs/server-key.pem
```

## Static Files
```
$ mkdir static
$ echo "I am not okay." > static/areyouok
$ serve&
$ curl http://localhost:8000/static/areyouok
2019-02-18T04:49:05.702926000Z 127.0.0.1 GET /static/areyouok "-" "curl/7.64.0" "-" "-"
I am not okay.
```

# Server access log format
Currently only the hardcode logging format is available. It is similar to an access log you might see from other servers, but is tailored to what sorts of information is useful from a penetration testing perspective.

| Timestamp | Remote IP | HTTP Method | URI | Referer | User Agent | Cookies | Authorization Header |
|---|---|---|---|---|---|---|---| 
| 2018-01-14T18:45:21.922171000Z | 127.0.0.1 | GET | /ping | "-" | "curl/7.54.0" | "-" | "eyJhbSomeJWT" |
