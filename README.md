# syslog_forwarder

Simply listen on one or many unix sockets and forward requests to a syslog server

This began as something to play around with rust, and turned into a tool that I use inside docker containers to get a `/dev/log` socket which talks to an external rsyslog server.

Usage is pretty simple:

```sh
syslog_forwarder -s /dev/log -s /some/other/socket/path -d <the address of the syslog server>:<the port>
```

It will then create those two sockets specified by the `-s` flag, and forward all reqeusts to the destination via udp
