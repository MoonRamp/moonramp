# Moonrampctl (Program)

## Configure the Default Sale Program

```
docker exec moonramp moonrampctl -a API_TOKEN program create -n moonramp-program-default-sale -P /home/moonramp/moonramp_program_default_sale.wasm -v test
```

You should see a result like

```
{
  "id": "a7e2344df06a4636a9bc8d09ba3346ce",
  "jsonrpc": "2.0",
  "result": {
    "createdAt": "2022-08-03T00:15:23.787162657Z",
    "description": null,
    "hash": "2CkZM8YwroAXMTCFjWdSW3t4FpK2TBArgfRz9KouR8z6",
    "name": "moonramp-default-sale-program",
    "private": true,
    "revision": 0,
    "url": null,
    "version": "test"
  }
}
```

The program is processed and then stored encrypted in the datastore ready for invocation. We are now ready to start storing wallets and processing crypto payments. For more information about programs capabilites see the [Programs](../../programs.md) section of this guide.

To get more info from `moonrampctl` on programs run the following.

```
docker exec moonramp moonrampctl help program
```

To get more info on any program subcommand run the following.

```
docker exec moonramp moonrampctl help program SUBCOMMAND
```
