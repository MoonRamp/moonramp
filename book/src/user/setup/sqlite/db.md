# Sqlite

## Run a bitcoin node in regtest mode

```
docker run --rm --name bitcoin -d moonramp/bitcoin:0.1.3-v23 -regtest
```

## Setup the Sqlite DB File

Create a docker volume and set the correct file permissions.

```
docker volume create moonramp
```

```
docker run --rm -v moonramp:/db busybox /bin/sh -c 'touch /db/moonramp.db && chown -R 1337:1337 /db'
```

Run the migrations to setup the db.

```
docker run -v moonramp:/home/moonramp/db --entrypoint=moonramp-migration moonramp/moonramp:0.1.18 -u "sqlite://db/moonramp.db" migrate
```
