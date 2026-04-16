test:
  cargo test -- --nocapture

run: 
  cargo run -- test.csv


run-release: 
  cargo run --release -- test.csv
