# Matthew stdio protocol, version 1

Run `matthew-app` once per analysis. It reads one JSON object from stdin through EOF (maximum 64 KiB), writes exactly one JSON result to stdout, and sends diagnostics to stderr. It does not serve HTTP or send callbacks. `Config.toml`, shared service tokens and callback HMAC secrets are no longer used.

Input:

```json
{"jobId":"unique-job-id","owner":"owner","repo":"repo","commitSha":"40-character-commit-SHA","token":"optional-installation-token"}
```

Success (exit 0):

```json
{"version":1,"jobId":"unique-job-id","commitSha":"40-character-commit-SHA","data":{"lorc":12,"counts":{"call_fn":3},"parsedFiles":1,"skippedFiles":0,"snapshotBytes":80},"error":null,"retryable":false}
```

`counts` contains all counters exported by the Matthew library. `lorc` counts lines in successfully parsed Rust files. Parse failures and invalid UTF-8 are skipped and reported; a repository whose Rust files all fail parsing returns a permanent error.

Analysis failure (exit 1) has `data: null`, an `error` string, and `retryable` indicating whether the consumer should retry. Malformed/oversized input exits 2 with diagnostics on stderr. Consumers must validate job ID, commit SHA, schema and exit code before accepting the output.

Limits apply to the pinned commit's files, not repository history:

- 1 GiB total Git blob bytes and 100,000 blob paths.
- 8 MiB per Rust source file and 256 MiB Rust source bytes in total.
- At most 1,000 non-recursive Tree API requests when recursive output is truncated.

Blob objects reused at multiple paths count once for each path. Submodules and symlink targets are not traversed. LFS pointer sizes count as blobs, and LFS contents are not downloaded. Missing blob sizes and unverifiable/truncated non-recursive trees fail closed.

The caller must enforce a wall-clock deadline and destroy the container after execution. The Ferris consumer uses GNU `timeout` to stop the full process group. Credentials pass through stdin and Git's process environment, never a clone URL or command argument.
