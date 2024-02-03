all:
	RUST_LOG=info cargo run --release -p demo
	RUST_LOG=info cargo run --release --example ramfs
	RUST_LOG=info cargo run --release --example devfs
	RUST_LOG=info cargo run --release --example procfs
	RUST_LOG=info cargo run --release --example fatfs
	RUST_LOG=info cargo run --release --example ext