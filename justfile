test:
  cargo test -- --nocapture

run: 
  cargo run -- transactions.csv


run-release: 
  cargo run --release -- test.csv
