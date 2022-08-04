# Docker

[![Docker Image](https://img.shields.io/docker/v/moonramp/moonramp?label=DockerHub)](https://hub.docker.com/r/moonramp/moonramp)

A [docker image](https://github.com/MoonRamp/moonramp/blob/master/Dockerfile) is built with every release that contains all MoonRamp bins and WASM programs.

The `ENTRYPOINT` of the image is set to the `moonramp` bin and the `CMD` must be overrided by the user.

Docker images run as a non-priviledged user `moonramp` and group `moonramp` with `uid:gid` `1337:1337`.

The following ports are defined via `EXPOSE` - `9370`, `9371`, and `9372`.

No other libaries or bins are included in the image. If you wish to build an image that includes other tooling you can use a [multi-stage](https://docs.docker.com/develop/develop-images/multistage-build/) build to copy the MoonRamp bins. The docker image is very lean at around 35 MB.

For more information on docker see [here](https://docs.docker.com/get-started/overview/).

## CLI

```
docker run moonramp/moonramp:0.1.18 help
```

## DockerFile

```
FROM moonramp/moonramp:0.1.18
```
