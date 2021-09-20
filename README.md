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
$ echo "Hello, world." | curl --data-binary @- http://localhost:8000/upload/test
```
## Server
```
$ target/release/serve
{"headers":{"accept":"*/*","host":"127.0.0.1:8000","user-agent":"curl/7.64.1"},"method":"GET","remote_addr":"127.0.0.1","ts":"2021-09-20T18:47:00.292Z","uri":"/ping"}
{"headers":{"accept":"*/*","content-length":"12","content-type":"application/x-www-form-urlencoded","host":"127.0.0.1:8000","user-agent":"curl/7.64.1"},"method":"POST","remote_addr":"127.0.0.1","ts":"2021-09-20T18:48:29.103Z","uri":"/upload/test"}
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
{"headers":{"accept":"*/*","host":"localhost:8000","user-agent":"curl/7.64.1"},"method":"GET","remote_addr":"127.0.0.1","ts":"2021-09-20T18:51:06.519Z","uri":"/static/areyouok"}
I am not okay.
```

# Server access log format
In version 0.3+ the server access log dumps fields of interest as JSON.
