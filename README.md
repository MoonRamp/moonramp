<div align="center">
  <h1>Moonramp Server</h1>

  <image src="./moonramp_icon.png">

  <h2>Start accepting crypto payments</h2>
  
  <div><p>
    Accept payments without fees or 3rd parties for your buisness or personal use today.
    This project is free and open-source software that fully embraces Satoshi's vision of peer-to-peer digital cash.
  </p></div>
  
  <div><p>
    <a href="https://dl.circleci.com/status-badge/redirect/gh/MoonRamp/moonramp/tree/master">
      <img src="https://dl.circleci.com/status-badge/img/gh/MoonRamp/moonramp/tree/master.svg?style=shield"/>
    </a>
    <a href="https://github.com/moonramp/moonramp/releases/">
      <img src="https://img.shields.io/github/v/release/moonramp/moonramp"/>
    </a>
    <a href="https://hub.docker.com/repository/docker/moonramp/moonramp">
      <img src="https://img.shields.io/docker/v/moonramp/moonramp"/>
    </a>
    <a href="https://github.com/moonramp/moonramp/blob/master/LICENSE">
      <img src="https://img.shields.io/github/license/moonramp/moonramp"/>
    </a>
  </p></div>
</div>

## WORK IN PROGRESS

<b>Early alpha state. Not all features are ready yet</b>

## Features
* BTC, BCH, ETH (+ERC-20 tokens), and XMR support
* Supports hot and cold wallets
* HD wallets
* Multi-layered encryption schema for SAFU funds
* Multi-tenant support
* Role base access control
* JsonRpc API
* Programmable contract system (Wasm based)
* SQL backend support (Mysql, Postgres, SQLite)
* Stand-alone reproducable statically linked binaries

## Install

### Download pre-built binaries

https://github.com/MoonRamp/moonramp/releases

### Docker Image

```
FROM moonramp/moonramp:0.1.0
```

### Build
#### Step 1

[Install Rust](https://www.rust-lang.org/)

#### Step 2

```
git clone git@github.com:MoonRamp/moonramp.git
```

#### Step 3

```
cd moonramp && cargo build --release
```

## Setup guide

## Donate

If you want to contribute to the project you can donate monero or bitcoin.
Donations will fund the development and marketing of the project.

### Monero (XMR)
<image width="256" src="./moonramp_monero.jpeg">
<p>468ZeZpnUdfXjkzvzt1QBcA5SU1coDg1z7CtKhzixfQabT1zUQt6YivN7XAAGbUzt4i6hXqkNTc82TzAG4FiLag1GK7xPSk</p>

### Bitcoin (BTC)
<image width="256" src="./moonramp_bitcoin.jpeg">
<p>bc1qefwe0jnue2327zjpef8e80ndyt24xjsqgt33ek</p>
