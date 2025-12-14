push-gateway CLI
===

To try things out using the build environment, first prepare a data file for pushing:

```
cat > /tmp/data << EOF
# HELP cloud_edge_wg_secs_since_last_handshake The number of seconds since we last received a Wireguard handshake from a peer.
# TYPE cloud_edge_wg_secs_since_last_handshake gauge
cloud_edge_wg_secs_since_last_handshake{client_pub_key="h/9Aa9yzJOlbbWWsb18xRPqZh7QRkpgqmYnz8Cy="} 1765685748
cloud_edge_wg_secs_since_last_handshake{client_pub_key="s7iFDhrtuudO7XNJP7eM/J6KkQrv4tg8/1Y="} 17
# EOF
EOF
```

...then run the program (assuming workspace dir):

```
RUST_LOG=info cargo run --bin push-gateway -- --push-http-path=/tmp/push-gateway.sock
```

...then, from another terminal, send data to the socket using curl:

```
curl \
  -v \
  --data-binary "@/tmp/data" \
  --unix-socket /tmp/push-gateway.sock \
  http://localhost/metrics/job/1
```
