# Dump, load, restart

## Dumping

To preserve your Devnet instance for future use, these are the options:

- Dumping on exit (handles Ctrl+C, i.e. SIGINT; doesn't handle SIGKILL):

```
$ starknet-devnet --dump-on exit --dump-path <PATH>
```

- Dumping after each block:

```
$ starknet-devnet --dump-on block --dump-path <PATH>
```

- Dumping on request, which requires providing `--dump-on request` on startup. You can also dump on request if you specified any of the other `--dump-on` modes.

### Dumping on request

You can request dumping by sending `POST` to `/dump` or via JSON-RPC. An optional file path can be provided in the request or on startup via `--dump-path <FILE>` (the HTTP request parameter takes precedence). If no dumping path is specified, the dump is included in the response body. This means that if you request dumping via [`curl`](https://curl.se/), it will be printed to STDOUT, which you can then redirect to a destination of your choice.

```
$ starknet-devnet --dump-on <MODE> [--dump-path <FILE>]
```

- No body parameters:

```
POST /dump
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_dump"
}
```

- With a custom path:

```
POST /dump
{
  // optional; defaults to the path specified via CLI if defined
  "path": <PATH>
}
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_dump",
    "params": {
        // optional; defaults to the path specified via CLI if defined
        "path": <PATH>
    }
}
```

## Loading

To load a preserved Devnet instance, the options are:

- Loading on startup (note the argument name is not `--load-path` as it was in Devnet-py):

```
$ starknet-devnet --dump-path <PATH>
```

- Loading on request, which replaces the current state with the one in the provided file. It can be done by sending `POST` to `/load` or via JSON-RPC:

```
POST /load
{ "path": <PATH> }
```

```
JSON-RPC
{
    "jsonrpc": "2.0",
    "id": "1",
    "method": "devnet_load",
    "params": {
        "path": <PATH>
    }
}
```

### Loading disclaimer

Currently, dumping produces a list of reproducible Devnet actions (state-changing requests and transactions). Conversely, loading is implemented as the re-execution of transactions from a dump. This means that timestamps of `StarknetBlock` will be different on each load. This is due to the nature of Devnet's dependencies, which prevent Devnet's state from being serialized.

Dumping and loading are not guaranteed to work across versions. I.e. if you dumped one version of Devnet, do not expect it to be loadable with a different version.

If you dumped a Devnet utilizing one class for account predeployment (e.g. `--account-class cairo0`), you should use the same option when loading. The same applies for dumping a Devnet in `--block-generation-on demand` mode.

## Restarting

Devnet can be restarted by making a `POST /restart` request (no body required) or `JSON-RPC` request with method name `devnet_restart`. All of the deployed contracts (including predeployed), blocks and storage updates will be restarted to the original state, without the transactions and requests that may have been loaded from a dump file on startup.

## Docker

To enable dumping and loading with dockerized Devnet, you must bind the container path to the path on your host machine.

This example:

- Relies on [Docker bind mount](https://docs.docker.com/storage/bind-mounts/); try [Docker volume](https://docs.docker.com/storage/volumes/) instead.
- Assumes that `/path/to/dumpdir` exists. If unsure, use absolute paths.
- Assumes you are listening on `127.0.0.1:5050`.

If there is `mydump` inside `/path/to/dumpdir`, you can load it with:

```
docker run \
  -p 127.0.0.1:5050:5050 \
  --mount type=bind,source=/path/to/dumpdir,target=/path/to/dumpdir \
  shardlabs/starknet-devnet-rs \
  --dump-path /path/to/dumpdir/mydump
```

To dump to `/path/to/dumpdir/mydump` on Devnet shutdown, run:

```
docker run \
  -p 127.0.0.1:5050:5050 \
  --mount type=bind,source=/path/to/dumpdir,target=/path/to/dumpdir \
  shardlabs/starknet-devnet-rs \
  --dump-on exit --dump-path /path/to/dumpdir/mydump
```
