## Docs

- Diesel:
	- [Diesel cli](https://github.com/diesel-rs/diesel/tree/master/diesel_cli#installation)
	- [Diesel getting started](http://diesel.rs/guides/getting-started/)
	- [example Diesel+sqlite](https://github.com/diesel-rs/diesel/tree/master/examples/sqlite/getting_started_step_3)


## Setup

1. if `linker 'cc' not found` => `sudo apt install build-essential`
1. Install Rust
1. DB:
	- `sudo apt-get install sqlite3 libsqlite3-dev`
	- `sudo apt install postgresql postgresql-contrib libpq-dev libmysqlclient-dev`
1. One of following options:
	- `cargo install diesel_cli`
	- `cargo install diesel_cli --no-default-features --features "postgres sqlite mysql"`
	- `cargo install diesel_cli --no-default-features --features "sqlite-bundled"`
1. `sudo apt-get install pkg-config libssl-dev` [?](https://docs.rs/openssl/0.10.23/openssl/)
1. `cargo build --release`
1. Edit `.env` file. Set up `RECAPTCHA_KEY` and `RUST_LOG` values.
1. Edit `.env` file. Set `CORS_ORIGIN` to "*" or "domain.zone".


Create DB-user & database:
- [guide](https://linuxize.com/post/how-to-install-postgresql-on-ubuntu-18-04/#creating-postgresql-role-and-database)
- `sudo -u postgres createuser owning_user`
- `sudo -u postgres createdb -O owning_user dbname`
- `sudo service postgres restart`
- Edit `.env` file. Set up `DATABASE_URL` value.


## Install

1. cargo install --force --path .


## Run

1. One of following options:
	- `nohup cargo run --release > LOG &`
	- `nohup ./target/release/bounty-server > LOG &`
	- `nohup bounty-server > LOG &`
1. `echo $! > PID`
1. `tail -f LOG`


### Kill

One of following options:
- ``kill `pidof cargo` ``
- ``kill `pidof bounty-server` ``
- ``kill `cat ./PID` ``

## Daemonize

[instructions](https://www.shellhacks.com/systemd-service-file-example/)

- `sudo touch /etc/systemd/system/bountyd.service`
- `sudo chmod 664 /etc/systemd/system/bountyd.service`

/etc/systemd/system/bountyd.service:

```ini
[Unit]
Description="Bounty Server"

[Service]
Type=simple
WorkingDirectory=/root/bounty-server/
PIDFile=/run/bountyd.pid
ExecStart=/root/.cargo/bin/bounty-server
ExecStop=/bin/kill -s QUIT $MAINPID

[Install]
WantedBy=multi-user.target
```

There `WorkingDirectory` point to dir with `.env` file.

- `sudo systemctl daemon-reload`
- `sudo systemctl start bountyd`
