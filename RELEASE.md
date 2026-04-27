# Releasing a new memc-rs version

The release process uses an annotated git tag containing the release notes and publishes the Docker image from that tagged commit.

## Creating a release

1. Create an annotated git tag with release notes:

```sh
git tag -a memcrsd-0.0.1c -m "Release memcrsd 0.0.1c\n\nRelease notes:\n- Add support for X\n- Fix Y bug"
git push origin memcrsd-0.0.1c
```

2. Build the Docker image from the git tag:

```sh
git checkout memcrsd-0.0.1c
docker pull rust
docker build -m 4096m -t memcrs/memc-rs:0.0.1c .
```

3. Tag and publish the Docker image:

```sh
docker tag memcrs/memc-rs:0.0.1c memcrs/memc-rs:latest
docker push memcrs/memc-rs:0.0.1c
docker push memcrs/memc-rs:latest
```

4. Optionally, if you want a separate patch or stable tag, create and push a lightweight alias:

```sh
git tag memcrsd-latest memcrsd-0.0.1c
git push origin memcrsd-latest
```