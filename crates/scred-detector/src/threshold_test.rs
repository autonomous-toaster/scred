//! Test different validation thresholds
//! This would be temporary to measure the benefit of increasing threshold

// Original thresholds:
//   8 cores: 4096 bytes (parallelizes on all 64KB chunks)
// 
// Proposed new thresholds:
//   8 cores: 256KB or 512KB (only parallelize on larger work)

// Reasoning:
// - Rayon thread pool overhead per iteration is fixed
// - Small work units (18 patterns on 64KB) don't amortize overhead
// - Need larger chunks to justify parallelization
// 
// Expected improvement:
// - If we make validation sequential: could be 5-10x faster
// - But parallel might still win on very large chunks
// - Optimal: sequential for < 256KB, parallel for >= 256KB

