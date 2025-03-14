# Private Token Wrapping

Provides an API that be used for private token wrapping

## Testing

* Start test validator

```shell
$> make test-validator
```

* Run tests

```shell
$> cargo test
```

## Wrapping Tokens

### Initialize Confidential Transfer Account

This will need to be done only once, and involves creating the confidential token account for the wrapped token mint.

To do this send a `POST` request to `http://example.com/confidential-balances/initialize` with the following payload

* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `elgamal_signature` is a message signed by the `authority` following the ElGamal signature from the confidential blink spec
* `ae_signature` is a message signed by the `authority` following the AE signature from the confidential blink spec

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "elgamal_signature": "3Cn8oMYkFjdZVDVvtFRbn9K8hMRZ4EWoFQ4m8LTpb9VL85hHXGwqpnDTtDPmBnsugppzNF7QfWvnG8NabGWN4d2V",
  "ae_signature": "47exZmEWHavGx7PXALPqjGn2qdh3dKfeqyt8Vj6amJJntdGTxuPURTKB9q4tRBgqiyosv4orjh6f6b1Cg3K8xwPk"
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request.

```json
{
  "transactions": [
    "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAYI+rdhoUWelWT7ijUTm1jMPw91zk0rKZJXk7AJ4U4ASVaEmsV9IxT8lBJsQolAkvEb/0PrNJa0l16F+RbrX5n0cgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABqfVFxh70WY12tQEVf3CwMEkxo8hVnWl27rLXwgAAAAG3fbh7nWP3hhCXbzkbM3athr8TYO5DSf+vfko2KGL/AhjdaziruooGms3TWgbp2pTzPY4wHRVk2wF0GVAAAAAjJclj04kifG7PRApFI4NgwtaE5na/xCEBI572Nvp+FnE8Sstk7CtexH58Qe9sM3ChaKbB5Wh7wy7sOWOphbLcAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAYGAAEABwIEAQEEBAEAAgADHQUABAQBBwMALxsChouPwrHzubhffjuolbzRXF2SfqQSgvdL0MVjirK1kEIY3/8GAAABAAAAAAABBQBhBDI1+sR8ND9KBgoNPEONdL3gjndURYVBiWpsjtfwdUUjGjBIFc6idcZEZImVaka/ALSVwa0VRbsX1x/fzGAF5AjF+zAs+9+CwUWUUcGayi4IxV0BVBigiG0TbKRXqxTCDg=="
  ]
}    

```

### Wrap The Tokens

Send a `POST` request to `http://example.com/private-wrapper/wrap` with the following payload. 

* `authority` is the public key of the wallet 
* `unwrapped_token_mint` is the token mint address of the unwrapped token (ie: USDT)
* `wrapped_token_mint` is the token mint of the wrapped mint as created via the spl token wrap program
* `unwrapped_token_program` is the program id of the token program which created `unwrapped_token_mint`
* `amount` is the amount of unwrapped tokens to wrap in lamports

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "unwrapped_token_mint": "GqxbzHAZrSaTGEqXcTCUiMR7bLUPrSCb4nZdqcKEkahv",
  "wrapped_token_mint": "3pQjCDCmaYGriwzz2M48WKkpXhMRRu3Q2Ghss8CqxRVx",
  "unwrapped_token_program": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
  "amount": 1
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request.

```json
{
  "transactions": [
    "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAUK+rdhoUWelWT7ijUTm1jMPw91zk0rKZJXk7AJ4U4ASVYp3IAoVvKs3kWHTVX2guZdh94TKi96xh2jbFTp4AwQJ32hcjOVj0fIE2NxZjn5g8Ej9sw5xKvzCO9FbwV5D14UhR58BbJsU4pua/UvEGLpf6XagAmTO/OUanxFEKOh3tHFy5FYJHc8ramA2J5VQrFRGYTsZs4JGPKVScRY3i5S6wXdgsd3+OvUnXsxAnsxgWy0bep7s7JFXPAhWGSSQblsBt324ddloZPZy+FGzut5rBy0he1fWzeROoz1hX7/AKkG3fbh7nWP3hhCXbzkbM3athr8TYO5DSf+vfko2KGL/JzWcOgns5hu0S1oSXZXhQJsSb+HXpqciMwOT2Fcno/I62oNsrC8NQAUhrgMjc9nGOkIHDqNDec4jdOzqZQncXEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAEFCQIBCAYHBAkDAAkBAQAAAAAAAAA="
  ]
} 
```


### Deposit Wrapped Tokens

To deposit your non confidential wrapped balance into the pending confidential balance send a `POST` request to `http://example.com/confidential-balances/deposit` with the following payload


* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `amount` is the amount of wrapped tokens in lamports to deposit

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "amount": 1
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request.

```json
{
  "transactions": [
    "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAIE+rdhoUWelWT7ijUTm1jMPw91zk0rKZJXk7AJ4U4ASVaEmsV9IxT8lBJsQolAkvEb/0PrNJa0l16F+RbrX5n0cgbd9uHudY/eGEJdvORszdq2GvxNg7kNJ/69+SjYoYv8xPErLZOwrXsR+fEHvbDNwoWimweVoe8Mu7DljqYWy3AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAECBAEDAAALGwUBAAAAAAAAAAk="
  ]
}    
```


### Apply Pending Balance

To apply the pending confidential balance send a `POST` request to `http://example.com/confidential-balances/apply` with the following payload

* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `elgamal_signature` is a message signed by the `authority` following the ElGamal signature from the confidential blink spec
* `ae_signature` is a message signed by the `authority` following the AE signature from the confidential blink spec

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "elgamal_signature": "3Cn8oMYkFjdZVDVvtFRbn9K8hMRZ4EWoFQ4m8LTpb9VL85hHXGwqpnDTtDPmBnsugppzNF7QfWvnG8NabGWN4d2V",
  "ae_signature": "47exZmEWHavGx7PXALPqjGn2qdh3dKfeqyt8Vj6amJJntdGTxuPURTKB9q4tRBgqiyosv4orjh6f6b1Cg3K8xwPk"
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request.

```json
{
  "transactions": [
    "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAIE+rdhoUWelWT7ijUTm1jMPw91zk0rKZJXk7AJ4U4ASVaEmsV9IxT8lBJsQolAkvEb/0PrNJa0l16F+RbrX5n0cgbd9uHudY/eGEJdvORszdq2GvxNg7kNJ/69+SjYoYv8xPErLZOwrXsR+fEHvbDNwoWimweVoe8Mu7DljqYWy3AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAECBAEDAAALGwUBAAAAAAAAAAk="
  ]
}    
```


### Get Balances

To get confidential and non confidential balances for the confidential wrapped mint send a `POST` request to `http://example.com/confidential-balances/balances` with the following payload



* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `elgamal_signature` is a message signed by the `authority` following the ElGamal signature from the confidential blink spec
* `ae_signature` is a message signed by the `authority` following the AE signature from the confidential blink spec

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "elgamal_signature": "3Cn8oMYkFjdZVDVvtFRbn9K8hMRZ4EWoFQ4m8LTpb9VL85hHXGwqpnDTtDPmBnsugppzNF7QfWvnG8NabGWN4d2V",
  "ae_signature": "47exZmEWHavGx7PXALPqjGn2qdh3dKfeqyt8Vj6amJJntdGTxuPURTKB9q4tRBgqiyosv4orjh6f6b1Cg3K8xwPk"
}    
```

The response will be a JSON object with the following fields

* `pending_balance` is the confidential balance waiting to be applied
* `available_balance` is the decrypted confidential balance
* `non_confidential_balance` is the non confidential balance

```json
{
  "pending_balance": 0.0,
  "available_balance": 1e-9,
  "non_confidential_balance": 0.00001
}    
```