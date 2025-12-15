node-exporter
===

A binary that serves to illustrate integration with `push-gateway`, and also provide what we
deem as "essential" system metrics. The `node-exporter` functionality is similar to that of
the Prometheus Node Exporter, but deliberately less flexible.

The `node-exporter` is designed to work on the smallest of embedded Linux environments with
a resident memory size of less than 5MiB.

To try things out, run the program (assuming workspace dir):

```
RUST_LOG=info cargo run --bin node-exporter -- --push-http-path=/tmp/push-gateway.sock
```

...then, from another terminal, start up the `push-gateway` and query its `/metrics`. The
metrics should be updated periodically.
