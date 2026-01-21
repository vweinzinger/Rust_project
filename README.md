# Tetris (web_client)

## Run locally
```bash
trunk serve --open
```

## Build a hostable bundle
Trunk outputs static files into `dist/`.

```bash
trunk build --release
```

Upload the contents of `dist/` to any static webserver.

### Hosting under a sub-path
If you host under a subdirectory (e.g. `/tetris/`), build with:

```bash
trunk build --release --public-url /tetris/
```
