.PHONY: test-validator
test-validator:
	solana-test-validator --reset --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb fixtures/token_2022.so --bpf-program PtwjzDzqbJr41iHYy8KG3Jb8VcwgRagJWj9Gk3Jg9f9 fixtures/spl_token_wrap.so --mint 6zdA4K3awdNX1TNAiwz9Xkzk2W9Hsw4g2jJabVFG7AP

.PHONY: bin
bin:
	cargo build --bin private-wrapper-cli

.PHONY: bin-release
bin-release:
	cargo build --release --bin private-wrapper-cli