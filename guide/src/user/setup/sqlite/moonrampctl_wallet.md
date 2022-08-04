# Moonrampctl (Wallet)

MoonRamp supports both hot and cold wallets. By default all BTC and BCH wallets support [BIP32 HD Wallets](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki). For XMR [view-only](https://www.getmonero.org/resources/user-guides/view_only.html) wallets are supported. ETH and ETC wallets support [ERC-20 Tokens](https://ethereum.org/en/developers/docs/standards/tokens/erc-20/).

Let's create a new BTC wallet to accept payment from a customer.

```
docker exec moonramp moonrampctl -a API_TOKEN wallet create -w hot -t btc
```

You should see something like this.

```
{
  "id": "7edc7b3b369e44598e0c5a0497aed044",
  "jsonrpc": "2.0",
  "result": {
    "createdAt": "2022-08-03T15:58:29.890989554Z",
    "hash": "2kg8XtHv1t5e5soBBFTGehn32sUwuqDmzyJjBtCg5K6q",
    "network": "Regtest",
    "pubkey": "tpubD6NzVbkrYhZ4WgTtjp7GR15TdZ35AUBAuxi8rxYTNTsLGf3j9AGYZrjwbLH5xDsJr3RE4vXxFK44fkRyA3UUBGRRDhYfagAu3vsntG9DTAb",
    "ticker": "BTC",
    "walletType": "Hot"
  }
}
```

Great job taking financial independence! We now have a MoonRamp managed BTC wallet to accept payments. As mentioned in the [Master Key Encryption Key](./../mkek.md) section this wallets [mnemoic code](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki) is stored with several layers of encryption. Mnemic codes are exportable and only the operator of the MoonRamp server has access to this information.


## Impl Details

### Bitcoin
MoonRamp uses the following libaries for bitcoin and bitcoin cash support

- [Rust Bitcoin](https://github.com/rust-bitcoin/rust-bitcoin)
- [Rust BIP39](https://github.com/rust-bitcoin/rust-bip39)
- [Rust RPC](https://github.com/rust-bitcoin/rust-bitcoincore-rpc)

### Monero

TODO

### Ethereum

TODO
