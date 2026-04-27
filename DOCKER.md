# Docker image

Docker image is available at docker hub: [https://hub.docker.com/r/memcrs/memc-rs](https://hub.docker.com/r/memcrs/memc-rs)

## Building docker image

Docker image contains only one binary, so image is pretty small(~8MB). To be able to build docker image additional memory needs to be granted to container that builds final image. Building docker image is divided in 2 stages. In stage one rust:latest image is used to compile static binary and the second stage contains just copies built binary into to final image.

To build docker image memcrsd sources have to be cloned and `docker build -m 512m .` command executed:

```sh
git clone git@github.com:memc-rs/memc-rs.git memc-rs
cd memc-rs
docker build -m 4096m .
```

## Publishing docker image

```sh
git checkout memcrsd-0.0.1b
docker pull rust
docker build -m 4096m -t memcrsd .
docker images
# tag docker image
docker tag 769dba683c8b memcrs/memc-rs:0.0.1b
docker tag memcrs/memc-rs:0.0.1b memcrs/memc-rs:latest
docker push memcrs/memc-rs:latest
docker push memcrs/memc-rs:0.0.1b
```

## Getting docker image from docker hub

To get latest version of memcrsd run following command:

```sh
docker image pull memcrs/memc-rs:latest
```

If you want specific version please take a look at available tags: [https://hub.docker.com/r/memcrs/memc-rs/tags](https://hub.docker.com/r/memcrs/memc-rs/tags)

## Running docker image

```sh
docker run -p 127.0.0.1:11211:11211/tcp -d memcrs/memc-rs
```
