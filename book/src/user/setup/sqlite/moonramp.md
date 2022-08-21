# Server Setup (Sqlite)

## Create the Master Merchant

The following command will generate a new merchant entry with the provide metadata and return a json blob.

```
docker run -v moonramp:/home/moonramp/db --entrypoint=moonramp-migration moonramp/moonramp:0.1.18 -u "sqlite://db/moonramp.db" create-merchant -n "My Merchant" -a "An Address" -e "email@example.com" -p "12223334444"
```

To run the next set of commands we need to come up with a secure [Master Key Encryption Key](../mkek.md). We recommend at least a 32 character random password. For our example we will use: `an example very very secret key.`

Using the `hash` from the `create-merchant` command replace `MERCHANT_HASH` and with your password replace `MKEK` in the following command to issue an API token for the master merchant. This API token has complete access to the merchant master account.

```
docker run -v moonramp:/home/moonramp/db --entrypoint=moonramp-migration moonramp/moonramp:0.1.18 -u "sqlite://db/moonramp.db" create-api-token -m MERCHANT_HASH -M MKEK
```

The api token record will be in the `token` field and the `Bearer Auth` string will be in the `api_credential` field of the json blob returned. The secret that is generated and encoded into the `api_credential` field is not stored in the database and if lost you will need to regenerate a new token.

## Run the Server

```
docker run --rm --name moonramp --link bitcoin -v moonramp:/home/moonramp/db -d moonramp/moonramp:0.1.18 node -u "sqlite://db/moonramp.db" -n example-node-id -m MERCHANT_HASH -M MKEK -N regtest
```

## Verify Servers

```
docker ps
```

You should see two docker containers running (`bitcoin`, `moonramp`). The output should look like the following.

```
CONTAINER ID   IMAGE                        COMMAND                  CREATED         STATUS         PORTS                                                      NAMES
4bd611af09c6   moonramp/moonramp:0.1.18     "moonramp node -u sq…"   2 minutes ago   Up 2 minutes                                                              moonramp
633e8855d98d   moonramp/bitcoin:0.1.3-v23   "bitcoind -regtest"      2 minutes ago   Up 2 minutes   8332/tcp, 18332/tcp, 18443/tcp, 38332/tcp                  bitcoin
```

Congrats! You now have a MoonRamp server running using sqlite as a data store and connected to a bitcoin node in regtest mode.

Continue to the [CLI Section](../moonrampctl.md)
