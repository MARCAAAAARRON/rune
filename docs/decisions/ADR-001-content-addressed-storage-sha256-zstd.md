# ADR-001: Content-Addressed Storage using SHA-256 and Zstandard Compression

## Status
Accepted

## Date
2026-10-15

## Context
Rune requires an immutable, content-addressed object storage model to represent snapshots of file versions, directory trees, and commit graphs. The design must satisfy several key criteria:
1. **Collision Resistance:** Protection against practical preimage and collision attacks (which have affected older hash functions like MD5 and SHA-1).
2. **Deterministic Addressing:** Identical payloads across files, trees, or commits must yield identical hashes to achieve automatic deduplication.
3. **High-Performance Compression:** Fast compression and decompression throughput for high repository I/O rates without inflating disk or cloud egress usage.
4. **Resilience to Decompression Attacks:** Mitigating decompression bomb vulnerabilities (e.g., zip-bomb style payloads) when fetching or reading untrusted objects.

## Decision
We chose **SHA-256** for content hashing and **Zstandard (`zstd`)** for on-disk and over-the-wire object compression:
- **Object ID Construction:** `ObjectId = SHA-256(object_type || '\0' || length || '\0' || content)`.
- **Zstandard Compression:** All objects written to `.rune/objects/` and remote buckets are compressed using `zstd::stream` at default speed/compression balance (level 3).
- **Directory Sharding:** Stored on disk under `objects/<hash[0:2]>/<hash[2:]>`, preventing filesystem performance degradation from flat directories with hundreds of thousands of entries.
- **Decompression Limit Guard:** Codec decoders enforce a strict maximum decompressed payload limit (default 100 MB per object) to mitigate memory exhaustion from malicious inputs.

## Alternatives Considered

### SHA-1 (Git Legacy Standard)
- **Pros:** Native Git compatibility; well-known ecosystem tool tooling.
- **Cons:** Broken collision resistance demonstrated in practice (e.g., SHAttered attack); requires complex transition protocols (like Git's ongoing SHA-256 transition).
- **Rejected because:** Rune is a clean-slate VCS with no Git backward-compatibility requirement. Starting with SHA-256 eliminates collision attack vectors from day one.

### BLAKE3
- **Pros:** Extremely fast hashing throughput on modern AVX-512 and multi-threaded architectures.
- **Cons:** Less ubiquitous in cloud storage metadata ecosystems compared to standard SHA-256.
- **Rejected because:** SHA-256 hardware acceleration is universal across modern CPU platforms and provides cryptographic guarantees aligned with industry compliance standards.

### Gzip / Deflate / Zlib
- **Pros:** Ubiquitous, standard HTTP compression algorithm.
- **Cons:** Slower decompression speeds, substantially lower compression ratios compared to modern Zstandard.
- **Rejected because:** Zstandard offers up to 3x-5x faster decompression speeds and higher compression ratios, directly improving cold-read and fetch performance.

## Consequences
- **Security:** Immutable object references are cryptographically secure against collision tampering.
- **Interoperability:** Rune `.rune` repositories cannot be directly read by standard Git tooling without an explicit converter.
- **Performance:** Compression and decompression overhead is minimal during high-frequency commit and checkout workflows.
- **Allocation Bounding:** All decompression code paths are protected against unbounded memory allocations.
