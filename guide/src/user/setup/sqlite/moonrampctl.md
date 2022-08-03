# Moonrampctl

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

## Validate Moonramp

`API_TOKEN` is the `token` field return from the json blob output of the `moonramp-migration create-api-token` command.

```
docker exec moonramp moonrampctl -a API_TOKEN program version
```

You should see result like

```
{
  "id": "69e78dd9e523458d92a889fa39ff0b82",
  "jsonrpc": "2.0",
  "result": "0.1.18"
}
```
