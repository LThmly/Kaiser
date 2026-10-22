# Replication, without the mystery

## Asynchronous replication

Replication copies data across multiple nodes. A leader accepts writes and sends updates to replicas. With asynchronous replication, the leader acknowledges a write before every replica has received it. This reduces write latency, but a replica can temporarily serve stale reads.

## Synchronous replication

Synchronous replication waits for an acknowledgement from the required replicas before confirming a write. Waiting increases write latency. If a required replica is unavailable, writes may be delayed or rejected.

## Replication lag

Replication lag is the delay between a write on the leader and its application on a replica. Reading from a lagging replica can return old data. Lag can grow when the network is slow or a replica cannot keep up with writes.

## Read your writes

Read-your-writes consistency ensures you can see your own completed updates. One approach is to route your reads to the leader after you write. Another is to track a write position and only read from replicas that have reached that position.

## A useful tradeoff

Serving reads from replicas distributes load and can put data closer to users. Whether stale data is acceptable depends on the use case. A public view count may tolerate some delay; confirming a just-completed account change usually should reflect the new value.
