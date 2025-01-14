# Nevermind

## Installation

1. Clone the repo
2. Install docker and run development containers
```sh
docker compose build
docker compose up -d
```
3. Install tools
```sh
cargo install sqlx-cli --no-default-features --features native-tls,postgres
cargo install cargo-watch
cargo install bunyan-rs
```
4. Copy .env.example to .env and fill values
5. Run database setup
```sh
make setup
```
6. Start dev server
```sh
make dev
```

## Development
1. Write features
2. Write tests
3. Fix / Refactor with working test

4. Before commit to pull request run
```sh
make format
make lint
make prepare
```
These help with code quality and sqlx requires 
offlines queries type for building
