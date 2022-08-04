# Master Key Encryption Key

MoonRamp uses a key encryption key scheme to uniquely encrypt all records independently of the underlying data store. The `Master Key Encryption Key` is a 256 bit key used to protect all data MoonRamp manages.

When the program boots it will use the provided `Master Key Encryption Key` to create or decrypt the current `Key Encryption Key`.
The `Master Key Encryption Key` will be purged from memory and the `Key Encryption Key` held in memory to decrypt `Encryption Key` records that are inturn used to decrypt specific record data.

This means that all data is secured via the `Master Key Encryption Key`. As such if this key is lost <b>ALL DATA</b> will be lost! To repeat this any hot wallets managed by the system will be <b>UNRECOVERABLE</b> if the `Master Key Encryption Key` is lost.

MoonRamp uses [Rust Crypto](https://github.com/RustCrypto).

## Rotating the Master Key Encryption Key

TODO

## Rotating the Key Encryption Key

TODO

## Shamir Secret Sharing

TODO: MoonRamp plans to support Shamir secret sharing to constitute a `Master Key Encryption Key` from multiple key shares.

## Encryption Stack

MoonRamp recommends a holistic approach to securing your data.

For the most security Merchants should:

* Maintain network firewall configurations to minimize network traffic to host running your data store and MoonRamp
* Run MoonRamp as a non-root user
* Run your data store on seperate hosts as a non-root user
* Enable full disk encryption for both your data store and MoonRamp hosts
* Enable transparent encryption for your data store
* Run all network traffic with TLS
* Utilize cold wallet support for high risk scenarios
