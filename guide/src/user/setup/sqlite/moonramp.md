# Server Setup (Sqlite)

## Create the Master Merchant

The following command will generate a new merchant entry with the provide metadata and return a json blob.

```
docker run -v moonramp:/home/moonramp/db --entrypoint=moonramp-migration moonramp/moonramp:0.1.18 -u "sqlite://db/moonramp.db" create-merchant -n "My Merchant" -a "An Address" -e "email@example.com" -p "12223334444"
```

Using the `id` from the `create-merchant` command replace `MERCHANT_ID` in the following command to issue an API token for the master merchant. This API token has complete access to the merchant master account.

```
docker run -v moonramp:/home/moonramp/db --entrypoint=moonramp-migration moonramp/moonramp:0.1.18 -u "sqlite://db/moonramp.db" create-api-token -m MERCHANT_ID
```

The api token will be in the `token` field of the json blob returned.

## Run the Server

```
docker run --rm --name moonramp --link bitcoin -v moonramp:/home/moonramp/db -d moonramp/moonramp:0.1.18 node -u "sqlite://db/moonramp.db" -n example-node-id -m MERCHANT_ID -M "an example very very secret key." -N regtest
```

Congrats! You now have a MoonRamp server running using sqlite as a data store and connected to a bitcoin node in regtest mode.
