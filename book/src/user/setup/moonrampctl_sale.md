# Moonrampctl (Sale)

Now it's time to create an invoice and get paid! `WALLET_HASH` is the `hash` field returned from the json blob of the `moonrampctl wallet create` command in the previous section.

```
docker exec moonramp moonrampctl -a API_TOKEN sale invoice -H WALLET_HASH -c btc  -a 0.25
```

You should see something like

```
{
  "id": "c8ad22f454764bd7807c967353034d27",
  "jsonrpc": "2.0",
  "result": {
    "address": "bcrt1qhe8apwuv2dsr95yknu826qad3cdxvs323ygc80",
    "amount": 0.25,
    "createdAt": "2022-08-03T20:49:52.483993Z",
    "currency": "BTC",
    "expiresAt": "2022-08-03T21:04:52.483994Z",
    "hash": "HhfxubuY8e36GQNhQVBMuEzTWMZcpfA7nzC12REuNf7e",
    "invoiceStatus": "Pending",
    "network": "Regtest",
    "pubkey": "tpubDCPc8xDGjqgqgW63nqsFiSTsJR8RRBjV4Npb5j3fMfu3Y9uTXB8AmbQpYKLNQGpGeJHmn6VYNHFoGpu76GT3JfabcJyaidsKNG2yq2PwvMH",
    "ticker": "BTC",
    "updatedAt": "2022-08-03T20:49:52.483994Z",
    "uri": "bitcoin:bcrt1qhe8apwuv2dsr95yknu826qad3cdxvs323ygc80;version=1.0&amount=0.25",
    "userData": null,
    "walletHash": "BvY3SinZbHkbnG1azbR8NcxzX88kVyc8TDzNxYa9TKbB"
  }
}
```

You have just created an invoice that expects payment of `0.25 btc` to the wallet `BvY3SinZbHkbnG1azbR8NcxzX88kVyc8TDzNxYa9TKbB` at the address `bcrt1qhe8apwuv2dsr95yknu826qad3cdxvs323ygc80` within `15 minutes`.

The fields `address` and `uri` are of particular note. `address` is a unique one-time address to receive payment. For Bitcoin, each call to generate a new invoice will generate a new address for a given wallet. The `uri` field is data that can be handled by a mobile OS url handler ([iOS](https://developer.apple.com/documentation/xcode/defining-a-custom-url-scheme-for-your-app), [Android](https://developer.android.com/training/app-links/deep-linking)). Most wallets support these type of uris when scanned as a QR code.

## Details About MoonRamp's Modeling

MoonRamp models payments as four objects.

An `Invoice`, `Sale`, `Refund`, and `Credit`.

### Invoice
An `Invoice` is a request to receive payment within a specific window of time for a specific amount to a specific address.

<i>Example: A merchant has a good or service they wish to collect payment for.</i>

### Sale
A `Sale` is a "capture" of an invoice that has been successfully funded in full.

<i>Example: A customer has fully funded an invoice and expects a good or service in return.</i>

### Refund
A `Refund` is a partial or full repayment of a specifc `Sale` to a specific address.

<i>Example: A customer wishes to return a good in exchange for their original payment.</i>

### Credit
A `Credit` is a repayment for a specific `Invoice` that was only partially funded to a specific address.

<i>Example: A customer only partially funded an invoice and should have their funds sent back.</i>

## Transfers
A `Refund` and `Credit` will make use of a `Transfer` which is a transaction from a specific wallet to a specific address. API tokens that have access to the wallet transfer API should be treated with extreme care.

<i>Example: A merchant wants to send funds from a hot wallet to an address.</i>
