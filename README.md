push-gateway
===

A [Rust-based](https://rust-lang.org/) push acceptor for caching the Prometheus metrics of local processes, 
and then making them available for scraping downstream. A unix domain socket is  available for other processes 
to stream out Prometheus metrics. Their keys are parsed by the push-gateway and retained in a cache, along with a
Time To Live (TTL). An http endpoint providing a `/metrics` path is available on a configurable port 
(`9091`) for downstream scraping.

By incorporating a TTL on each metric key, if a local process is upgraded and no longer supplies that
metric then it will naturally disappear. The TTL is one minute by default as it is conventional for
metrics to be updated by other processes at least once within 15 seconds.

The push-gateway is functionally comparable with the [Prometheus Push Gateway](https://github.com/prometheus/pushgateway),
but designed for embedded environments like those found in edge computing. The size of the cache is fixed to a
command-line option. By default, the cache will not exceed 10KiB of memory, which enables around 100 labelled metrics to be stored. 
The resident size of the push-gateway is not expected to exceed 2MiB for an ARM 64 bit target.

With the embedded target in mind, you can also be sure that this push-gateway works extremely well anywhere
and can be considered a replacement to the Prometheus Push Gateway. The impact of doing this means that the
processes wanting to push metrics must do so using a Unix Domain Socket instead of HTTP. However, [`socat`](https://man.freebsd.org/cgi/man.cgi?query=socat&sektion=1&manpath=FreeBSD+6.0-RELEASE+and+Ports)
can be used to forward http request bodies on to a Unix Domain Socket as a migratory step.

Motivation
---
Push gateways are a useful approach to collecting metrics from an unknown number of sources. Only
one well-known endpoint then needs to be configured and scraped by the Prometheus server or another collector,
instead of these having to know which endpoints to poll.

We did initially consider using the [Prometheus Push Gateway](https://github.com/prometheus/pushgateway) for our
embedded target. However, the Prometheus Push Gateway is understood to require around 128MiB of memory to run, which is 
too much for an embedded target (we have deployed to edge compute with 128MiB in total!).

The Prometheus Push Gateway also provides its own metrics which we will not leverage.

Finally, Unix Domain Sockets are hard to beat in terms of performance when conveying data between
processes. The Prometheus Push Gateway provides an HTTP endpoint instead, which is overkill for
inter-process communication. And then there's the garbage collection associated with Go...

In summary, we need a process to work in the smallest amount of memory while consuming the
smallest amount of CPU, and at the same time, assume a single-core microprocessor.

## Contribution policy

Contributions via GitHub pull requests are gladly accepted from their original author. Along with any pull requests, 
please state that the contribution is your original work and that you license the work to the project under the 
project's open source license. Whether or not you state this explicitly, by submitting any copyrighted material via 
pull request, email, or other means you agree to license the material under the project's open source license and 
warrant that you have the legal authority to do so.

## License

This code is open source software licensed under the [Apache-2.0 license](./LICENSE).

© Copyright [Cuprous P/L](https://www.cuprous.com.au/), 2025
