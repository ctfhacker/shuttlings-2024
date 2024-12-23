cargo watch -s 'clear' -x 'clippy --all-features --all-targets -- -D warnings -D clippy::pedantic' -x 'test -- --nocapture' -s 'shuttle run'
