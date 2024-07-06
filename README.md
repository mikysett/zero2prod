# Exercises from the book Zero to production in Rust

[The book website](https://www.zero2prod.com/index.html?country=Portugal&discount_code=VAT20)

The main program is an email newsletter server, the scope being to create a backend application using **Actix Web** and **Tokio**.

## Prerequisites

- Install `rustup`
- Run `cargo install --version="~0.6" sqlx-cli --no-default-features --features rustls,postgres`
- Install `docker`
- Install `PostgreSQL`,

## Run the application

- At root run `bash scripts/init_db.sh`
- This will run a local Postgres DB in a container (used for testing and compile time checks)