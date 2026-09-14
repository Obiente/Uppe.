# Implementation status

The consolidation integrates six pull requests, including the three drafts. Their original revisions were not independently ready to merge.

| PR | Integrated work | Original blockers addressed |
| --- | --- | --- |
| #23 | PeerUP crypto, distributed types, DHT operations | Real verification, signer binding, vote replay checks, bounded storage, formatting and Clippy |
| #24 | Rust orchestrator, checks, storage, P2P | Missing dependency, helper response routing, monitor reload lifecycle, target validation, per-monitor timeouts, retention |
| #25 | TUI views and event routing | UTF-8 truncation crash, Tab routing, wrong list navigation, stale hit regions and count underflow |
| #39 (draft) | Go API and shared contracts | Recreated missing module, entry point, configuration, schemas and clients; operator authentication; schema tests; removed ineffective settings writes |
| #40 (draft) | Editable status pages and anonymous view | Publication gate, failure versus not-found handling, unknown/stale health, atomic bindings, working browser forms |
| #41 (draft) | Signed event and attestation foundation | Durable transactional outbox, actor/key binding, append-only records, failure-path tests |

## Supported flow

Run an installation, authenticate, create HTTP/TCP monitors, read real checks and history, edit or pause monitoring, and explicitly publish/unpublish status pages. Configuration and result storage work across Rust and Go. Unsupported runtime settings fail explicitly.

## Remaining work

1. Implement resumable authenticated history transfer with per-source cursors and complete-batch acknowledgments. Existing DHT/query scaffolding does not provide offline recovery.
2. Exercise helper renewal, churn, floods, partitions and malicious peers on a wider test network. Open membership is intentional; signed votes alone are not a Sybil defense. Local health remains authoritative.
3. Design capability delegation, durable revocation distribution and admin-key operations. Locally pinned roots are mandatory for chain acceptance; bootstrap URLs cannot supply their own trust roots.
4. Build replicated signed status-page bundles and independent hosting. The public view currently needs the owner's running API.
5. Add notification delivery and custom HTTP checks through an explicit execution contract. The UI does not offer simulated settings for these.
6. Define signed audit archival and resource budgets for large installations. Raw result retention does not prune audit history.

Encrypted result envelopes now use version 2, authenticated metadata and corrected key conversion. Old experimental encrypted messages are rejected; restart participating nodes on the same version. Static owner keys do not provide forward secrecy against later owner-key compromise.

The standalone TUI reads persisted network snapshots. Live DHT requests need cross-process command transport; the current view reports that they are unavailable instead of showing a query as pending indefinitely.
