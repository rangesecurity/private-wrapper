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

## Initialize Confidential Transfer Account

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

## Wrap The Tokens

> Note: You must first initialize the confidential transfer account

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

## Deposit Wrapped Tokens

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

## Apply Pending Balance

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

## Get Balances

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

## Transferring Confidential Tokens

> Note: the recipient must first initialize a confidential transfer account
> Note: After transferring the recipient must apply the pending balance

To transfer confidential tokens you will need to generate three temporary keypairs used to store proof state. Label the keypair as follows

* `equality_proof_keypair`
* `range_proof_keypair`
* `ciphertext_proof_keypair`

After generating the keypairs send a `POST`  request to `http://example.com/confidential-balances/transfer` with the following payload

* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `elgamal_signature` is a message signed by the `authority` following the ElGamal signature from the confidential blink spec
* `ae_signature` is a message signed by the `authority` following the AE signature from the confidential blink spec
* `receiving_token_account` the ATA of the wrapped mint for the public key you want to transfer funds too
* `equality_proof_keypair` The base58 encoded private key of the equality proof keypair
* `range_proof_keypair` The base58 encoded private key of the range proof keypair
* `ciphertext_proof_keypair` The base58 encoded private key of the ciphertext proof keypair
* `amount` The amount of tokens to transfer in lamports

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "elgamal_signature": "3Cn8oMYkFjdZVDVvtFRbn9K8hMRZ4EWoFQ4m8LTpb9VL85hHXGwqpnDTtDPmBnsugppzNF7QfWvnG8NabGWN4d2V",
  "ae_signature": "47exZmEWHavGx7PXALPqjGn2qdh3dKfeqyt8Vj6amJJntdGTxuPURTKB9q4tRBgqiyosv4orjh6f6b1Cg3K8xwPk",
  "receiving_token_account": "AtLx7URpBXfsfnWLhxXUtngji5PnKCoR2GY3YWxQZsU9",
  "equality_proof_keypair": "PVVyq6JHhZySpAfdrTjiYaGrTcYtaJ87pMvxr7KK7bDLdyFPujbFfyyDyVjmXPZyxSbthh8LLEL36fcYSDeqgGf",
  "ciphertext_validity_proof_keypair": "3qYrL5FgBuPR5CJRAzvf7dk2o8J8rt8bTtMEnUo4EQ736ovymaVtbumT43SM8J1RHkVg8AqhRfvQxMHW2RVtWQ27",
  "range_proof_keypair": "59ceHzjfZS1RUUg5ZVF1uxxdPo7rzWPpKZBeK5pq1aHAjSRoFohKLUKWAgCSQmtPe1B4rthTKjHyR7xAeUQXHmX6",
  "amount": 1
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request. The very first transaction in the array also needs to be signed by the `equality_proof_keypair`, `range_proof_keypair` and `ciphertext_proof_keypair`.

You must ensure that each transaction has confirmed before sending the next transaction. The very final transaction closes the equality proof, range proof, and ciphertext proof accounts refunding the rent to the `authority` specified in the request payload.

```json
{
  "transactions": [
    "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"
  ]
}    
```

## Withdrawing Confidential Tokens

To withdraw confidential tokens you will need to generate two temporary keypairs used to store proof state. Label the keypair as follows

* `equality_proof_keypair`
* `range_proof_keypair`

After generating the keypairs send a `POST`  request to `http://example.com/confidential-balances/withdraw` with the following payload

* `authority` is the public key of the wallet
* `token_mint` is the mint address of the confidential wrapped mint
* `elgamal_signature` is a message signed by the `authority` following the ElGamal signature from the confidential blink spec
* `ae_signature` is a message signed by the `authority` following the AE signature from the confidential blink spec
* `receiving_token_account` the ATA of the wrapped mint for the public key you want to transfer funds too
* `equality_proof_keypair` The base58 encoded private key of the equality proof keypair
* `range_proof_keypair` The base58 encoded private key of the range proof keypair
* `amount` The amount of tokens to withdraw from the confidential balance

```json
{
  "authority": "Hsh7Fp27e3JbQQog9i1nzF6qY8fdWHTbF7RW1xzuLx5T",
  "token_mint": "EFnCaHgGto1NNk6Ym7TCoxdwKF24u2CYMSzaHHJ6pbFu",
  "elgamal_signature": "3Cn8oMYkFjdZVDVvtFRbn9K8hMRZ4EWoFQ4m8LTpb9VL85hHXGwqpnDTtDPmBnsugppzNF7QfWvnG8NabGWN4d2V",
  "ae_signature": "47exZmEWHavGx7PXALPqjGn2qdh3dKfeqyt8Vj6amJJntdGTxuPURTKB9q4tRBgqiyosv4orjh6f6b1Cg3K8xwPk",
  "amount": 1,
  "equality_proof_keypair": "4ZS9bedJkNCAgcV7fz19W8qirhMo4gBc1gQpxHLsJvadxeSFhPV12CU6YTGhL6MMWX2mzxcJpFNsySQb6t7sx3qy",
  "range_proof_keypair": "EAc1zWBSFuSdtXUhMTRXiK7NdRgXBcCs1TVtyg13iDkYw9K6qnov3GBE5X11Xz3rmG8zrC6hX6MX1cmcuiYYCDi"
}    
```

The response will be an aray of bincode serialized, base64 encoded transactions that need to be parsed, and signed by the `authority` specified in the request. The very first transaction in the array also needs to be signed by the `equality_proof_keypair`, `range_proof_keypair`

You must ensure that each transaction has confirmed before sending the next transaction. The very final transaction closes the equality proof, and range proof accounts refunding the rent to the `authority` specified in the request payload.

```json
{
  "transactions": [
    "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"
  ]
}    
```

## Unwrapping Tokens

> Note: Before you unwrap tokens, you must first withdraw them from your confidential balance into your non confidential balance

Send a `POST` request to `http://example.com/private-wrapper/unwrap` with the following payload.

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
