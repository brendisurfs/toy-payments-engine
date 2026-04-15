test:
  cargo test -- --nocapture

run-debug: 
  cargo run -- test.csv


run-release: 
  cargo run --release -- test.csv
