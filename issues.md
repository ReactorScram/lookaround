**Issues**

Just doing ULIDs cause `rusty_ulid` makes it easy.

# 01FP9843V1J3H9JMHXFPJSV2QJ

Have to disable VirtualBox virtual interface thingy to make it work.

Might also misbehave on systems with both Ethernet and WiFi connections.

I think this is because the `UdpSocket`, when I tell it to bind to
`0.0.0.0`, doesn't actually bind to all interfaces, it picks an interface
and binds to it.

I don't have any systems at home to replicate this on. And if I have
to poll multiple sockets, I'll probably just drag in Tokio even
though I was hoping not to use it - It's nicer than threading.

I don't think Tokio has a way to iterate over network interfaces
and get their IPs, so I might have to find another dependency
for that. I think on Linux I can get it from `/sys/class/net` but
I can't remember the trick for that. I think last time I did this
(for that work project) I just punted to Qt.
