# confidential-blink-api
API for confidential blinks


## Testing

* Dump spl token 2022 from devnet

```shell
$> solana --url devnet program dump TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb ata.so
```

* Start test validator

```shell
$> solana-test-validator --reset --bpf-program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb ata.so --mint 6zdA4K3awdNX1TNAiwz9Xkzk2W9Hsw4g2jJabVFG7APS
```

* Run tests

```shell
$> cargo test
```