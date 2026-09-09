# Verification gates

- G1 (ACTIVE from PP-01): fmt clean, clippy clean with restriction lints,
  all tests pass, release build, forbidden-dependency check, unsafe
  tripwire, log determinism. Run: `bash scripts/check-g1.sh`
- G2 (future): html5lib tokenizer + tree-construction suites at 100%.
- G3 (future): curated CSS 2.1 reftest subset.
- G4 (future): test262 ES5.1 at >= 99% with reviewed allowlist.
- G5 (future): golden PNG snapshots, byte-stable.
- G6 (future): full-run determinism, 10 runs, identical hashes.
- G7 (future): 1M fuzz executions per parser target, zero crashes.
- G8 (future): 60fps scroll on the era fixture; incremental reflow < 10ms.
