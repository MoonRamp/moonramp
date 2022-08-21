# Moonrampctl

## Validate Moonramp

`API_TOKEN` is the `api_credential` field returned from the json blob output of the `moonramp-migration create-api-token` command run during your desired database setup ([Sqlite](./sqlite/db.md), [Postgres](./postgres/db.md), [CockroachDB](./cockroachdb/db.md), [MySql](./mysql/db.md)).

```
docker exec moonramp moonrampctl -a API_TOKEN program version
```

You should see something like

```
{
  "id": "69e78dd9e523458d92a889fa39ff0b82",
  "jsonrpc": "2.0",
  "result": "0.1.18"
}
```
