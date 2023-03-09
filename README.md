# ⚙️ starter-rust-rest-api
*Work In Progress*

# 🚀 Getting Started
- Download Rust & the following CLI tools
	- `cargo install sea-orm-cli`
	- `cargo install sqlx-cli --no-default-features --features rustls,postgres`
- Download Docker Compose to host your database & the Redis session store
	- Run `docker compose up --build` to build your Docker image
- Rename `TODO.env` to `.env`
- Run `sqlx database create` to create a database in your Postgres instance
- Run `cargo run` to run the backend at localhost:8080

# 🧑‍🚀 Development
- `docker compose start`: Start your existing Docker image
- `cargo fmt`: Code formatting
- `cargo watch -x run`: Run the backend with hot reloading (Run `cargo install cargo-watch` first)

# 📆 Future Release Plans
- File upload route?