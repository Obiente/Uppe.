# PeerUP

**Decentralized, verifiable uptime monitoring.**

PeerUP is a distributed system where nodes monitor both their own services and assigned peers, providing signed uptime and latency data across the network.

---

## Overview

### Architecture

**1. Node**

- Monitors its own services.
- Monitors a subset of peer-assigned URLs.
- Signs and publishes monitoring results.

**2. Shared Storage**

- Results are written to a common database (centralized or distributed via DHT/IPFS).
- Each record includes:

  - `url`
  - `timestamp`
  - `status` (up/down)
  - `latency`
  - `monitor_id`
  - `signature`

**3. Coordination**

- Uses assignment logic (e.g. consistent hashing or round-robin).
- Ensures redundancy: every URL is monitored by multiple independent nodes.

**4. Frontend**

- Web-based dashboard for uptime, status breakdown, and latency analytics.
- Aggregates self and peer reports.

---

## Data Format

```json
{
  "url": "https://yourapp.com",
  "timestamp": "2025-06-01T12:00:00Z",
  "status": "up",
  "latency_ms": 198,
  "monitor_id": "peer-node-22",
  "signature": "base64-edsig..."
}
```

---

## Workflow

1. Node registers a URL to monitor.
2. Begins local checks (e.g. every 30s).
3. URL is added to the peer pool.
4. Peers begin independent monitoring.
5. All results are signed and published.
6. Frontend shows global status consensus.

---

## Trust Model

- All results are cryptographically signed.
- Nodes can be identified and audited.
- Future enhancements:

  - Reputation scoring.
  - Optional proof-of-work or staking model to prevent spam.
