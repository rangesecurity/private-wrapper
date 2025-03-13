# Confidential Blink API

Provides an API that can be used for confidential transfer blinks, and also as a general API for use with frontends to facilitate interaction with the confidential token extension

## Testing

* Dump spl token 2022 from devnet

```shell
$> solana --url devnet program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb token_2022.so
```

* Start test validator

```shell
$> solana-test-validator --reset --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb token_2022.so --mint 6zdA4K3awdNX1TNAiwz9Xkzk2W9Hsw4g2jJabVFG7AP
```

* Run tests

```shell
$> cargo test
```
