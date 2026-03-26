# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-03-26

### Added
- ✨ **SIMD Performance Optimization**
  - Portable Rust SIMD intrinsics for charset scanning
  - x86_64 SSE2 support
  - ARM64 NEON support
  - Scalar fallback for compatibility
  - Feature-gated: `--features simd-accel` (nightly only)
  - 29-48% micro improvement, 0.4-2.1% macro improvement
  
- **Enhanced Testing**
  - 7 new SIMD-specific unit tests
  - Comprehensive micro-benchmarks (3 groups, 13 test cases)
  - Macro-benchmarks with realistic detection scenarios
  - Prediction accuracy validation (95% accuracy)

- **Production Documentation**
  - SIMD_IMPLEMENTATION.md - Technical deep dive
  - SIMD_DEPLOYMENT.md - Production deployment guide
  - PROJECT_STATUS.md - Complete project status
  - RELEASE_NOTES_v0.2.0.md - Release notes

- **CI/CD Infrastructure**
  - GitHub Actions test matrix
  - Stable Rust + Nightly Rust jobs
  - Multi-platform testing (Ubuntu, macOS)
  - Automatic benchmark execution

### Changed
- Improved code quality
  - Cleaned up unused imports
  - Fixed unused variable warnings
  - Better Cargo.toml bench configuration
  - More idiomatic Rust patterns

- Enhanced pattern coverage validation
  - All 273+ patterns have bounded max_len
  - Pattern distribution analysis (22% charset, 78% regex)
  - Tier classification (Critical, Infrastructure, Services, etc.)

### Fixed
- Removed unused import: `crate::match_result::Match`
- Removed unused import: `std::os::raw::c_int`
- Fixed duplicate bench target warnings in Cargo.toml
- Cleaned up unused loop variables

### Performance
| Metric | Improvement |
|--------|-------------|
| Charset scanning (micro) | 29-48% |
| Boundary detection | 38-48% |
| Full detection (macro) | 0.4-2.1% |
| HTTP request redaction | +6.8% |

### Test Results
- ✅ 346/346 tests passing
- ✅ All platforms (x86_64, ARM64, scalar)
- ✅ Both Rust versions (stable, nightly)
- ✅ Zero regressions

---

## [0.1.0] - 2026-03-25

### Initial Release
- Complete secret pattern detection engine
  - 273+ secret patterns across 252 providers
  - Hierarchical pattern classification (4 types)
  - Multi-line pattern support (SSH keys, PGP, certificates)
  - Streaming redaction with character preservation

- Detection patterns by tier:
  - 87 Critical (AWS, GitHub, Stripe, etc.)
  - 124 Infrastructure (databases, APIs, cloud)
  - 20 API Keys (service credentials)
  - 22 Services (SaaS provider tokens)
  - 2 Generic patterns (JWT, generic API key)

- TLS MITM Proxy
  - Full HTTPS interception
  - Per-domain certificate caching
  - Request/response redaction
  - Corporate proxy support

- Character-Preserving Redaction
  - Keeps first 4 chars visible for context
  - Maintains length for pattern compatibility
  - Special handling for environment variables
  - Full redaction for SSH keys

- Streaming Architecture
  - Bounded lookahead (3-20KB per pattern)
  - Memory-efficient (<64KB typical)
  - Real-time redaction capable
  - Low-latency operations

- Testing & Quality
  - 338 comprehensive unit tests
  - Integration tests with 15+ scenarios
  - Edge case coverage
  - Pattern validation suite

- Documentation
  - Complete API documentation
  - Pattern analysis guides
  - Integration examples
  - Troubleshooting guides

### Deployment
- Docker support
- Multiple deployment modes (CLI, proxy, library)
- Comprehensive configuration options
- Production-ready logging

---

## Unreleased

### Planned Features
- [ ] Pattern audit & false positive reduction
- [ ] Production metrics collection
- [ ] GPU acceleration (long-term)
- [ ] Distributed redaction (long-term)
- [ ] ML-based pattern detection (research)

### Performance Roadmap
- [ ] Thread-local buffer pooling (10-15% improvement)
- [ ] Pattern trie optimization (20-30% improvement)
- [ ] Early-exit fast path (5-10% improvement)

### Pattern Coverage
- [ ] Additional cloud provider patterns
- [ ] More SaaS integrations
- [ ] Custom pattern support
- [ ] Pattern whitelisting

---

## Support

For help with any release:
- **Documentation**: See SIMD_IMPLEMENTATION.md, SIMD_DEPLOYMENT.md
- **Issues**: GitHub Issues
- **FAQ**: SIMD_DEPLOYMENT.md#faq

## Compatibility

### v0.2.0
- Rust: 1.70+ (stable), nightly for SIMD
- Platforms: Linux, macOS, Windows
- CPU: x86_64, ARM64, any (scalar fallback)
- Fully backward compatible with v0.1.0

### v0.1.0
- Rust: 1.70+
- Platforms: Linux, macOS, Windows (limited)
- CPU: Any (scalar only)

---

## Version History

| Version | Date | Status | Notes |
|---------|------|--------|-------|
| 0.2.0 | 2026-03-26 | 🟢 Stable | SIMD optimization + CI/CD |
| 0.1.0 | 2026-03-25 | 🟢 Stable | Initial release |
