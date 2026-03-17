# Docker Build Troubleshooting Guide

## Issue: Docker Build Timeout or Failure

**Error Message:**
```
error reading from server: EOF
rpc error: code = Unavailable desc = error reading from server: EOF
```

### Root Causes

1. **Insufficient Memory** (Most Common)
   - Docker Desktop default: 2GB RAM
   - Rust compilation needs: 4GB+ RAM
   - Fix: Increase Docker Desktop memory to 4-6GB

2. **Slow Network or Disk**
   - Cargo downloading dependencies
   - Docker layer caching issues
   - Fix: Check internet speed, ensure SSD storage

3. **Large Build Context**
   - Too many files being sent to Docker builder
   - Default: entire repository
   - Fix: Use `.dockerignore` file (provided)

4. **Timeout During Compilation**
   - Long compilation with limited parallelism
   - Default timeout: 10 minutes
   - Fix: Reduce parallelism (-j 1), increase timeout

### Quick Fix

**Step 1: Check Docker Desktop Settings**

1. Open Docker Desktop
2. Go to Settings → Resources
3. Increase Memory to 4GB or more (recommend 6GB)
4. Click "Apply & Restart"

**Step 2: Clean Docker Cache**

```bash
docker system prune -a
docker builder prune
```

**Step 3: Rebuild**

```bash
cd docker-compose
bash build-and-test.sh
```

### Solution: Optimized Dockerfiles

We've provided optimized Dockerfiles that:

1. **Use `-j 1` (single-threaded build)**
   - Reduces peak memory usage
   - Slower but more reliable on Docker Desktop

2. **Use `debian:bookworm-slim` runtime**
   - ~400MB instead of 1GB+ for `rust:latest`
   - Smaller final images

3. **Enable aggressive optimization**
   - `-C opt-level=z` (minimal size)
   - `-C strip=symbols` (remove debug info)
   - mold linker (faster than lld)

4. **Minimize build context**
   - `.dockerignore` excludes unnecessary files
   - Only copy Cargo files and source code

### Files Modified

**Dockerfile.proxy** (lines 16-18):
```dockerfile
# Changed from -j 2 to -j 1
# Added CARGO_BUILD_JOBS=1 and opt-level=z
RUN cargo build --release --package scred-proxy -j 1
```

**Dockerfile.mitm** (same changes)

**.dockerignore** (new file):
```
# Excludes 100+ MB of unnecessary files
target/
.git/
docs/
*.md
node_modules/
venv/
```

### Expected Build Times

| Environment | Memory | CPU | Time |
|-------------|--------|-----|------|
| Docker Desktop (optimized) | 4GB | M1/M2 | 5-8 min |
| Docker Desktop (optimized) | 6GB | M1/M2 | 3-5 min |
| Docker Desktop (original) | 2GB | M1/M2 | FAILS |
| GitHub Actions | 4GB | Standard | 10-15 min |

### Complete Build Process

```bash
# 1. Navigate to repo root
cd /path/to/scred

# 2. Run optimized build script
cd docker-compose
bash build-and-test.sh
```

This script will:
1. Check Docker status
2. Clean old containers
3. Build images with progress output
4. Start docker-compose stack
5. Wait for health checks
6. Run quick connectivity tests

### Manual Build (if script fails)

```bash
# 1. From repo root
cd /path/to/scred

# 2. Build images individually
docker build -t scred-proxy:latest \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  -f Dockerfile.proxy \
  --progress=plain \
  .

docker build -t scred-mitm:latest \
  --build-arg BUILDKIT_INLINE_CACHE=1 \
  -f Dockerfile.mitm \
  --progress=plain \
  .

# 3. Start stack
cd docker-compose
docker-compose up -d --wait
```

### Verify Build Success

```bash
# Check images exist
docker images | grep scred

# Check containers running
docker-compose ps

# Test services
curl http://localhost:8001/secrets.json    # fake-upstream
curl http://localhost:8000/get             # httpbin
curl http://localhost:9999/get             # scred-proxy
```

### Additional Troubleshooting

**If build still times out after memory increase:**

```bash
# Check Docker's actual memory limit
docker info | grep Memory

# Check system resources
docker stats  # (while building in another terminal)

# Increase timeout explicitly
timeout 600 docker-compose build scred-proxy
```

**If disk space is low:**

```bash
# Check disk usage
df -h

# Free up space
docker system prune -a --volumes
docker builder prune --all

# You need ~20GB free for full build
```

**If network is slow:**

```bash
# Use Cargo registry mirror (China users)
export CARGO_NET_GIT_FETCH_WITH_CLI=true

# Or configure ~/.cargo/config.toml
[registries.crates-io]
protocol = "sparse"
```

### Performance Tips

1. **SSD Storage** (Critical)
   - Docker build cache on SSD dramatically faster
   - NFS/SMB shares much slower

2. **Disable Antivirus Scanning**
   - Windows Defender slows Docker significantly
   - Add Docker Desktop folders to exclusions

3. **Docker Desktop Settings**
   - Memory: 4-6GB minimum, 8GB recommended
   - CPU: 4+ cores
   - Swap: 2-4GB
   - Disk image: 50GB+ size

4. **Build in Sequence**
   - Build scred-proxy first (finishes faster)
   - Then scred-mitm (uses cached layers from proxy build)

### Support

If issues persist after trying these steps:

1. Check docker-compose logs: `docker-compose logs --tail=100`
2. Check system resources: `docker stats`
3. Verify Docker Desktop installation
4. Try Docker CLI build with verbose output: `--progress=plain`
5. Check free disk space: `df -h` (need 20GB+)

**Most common solution:** Increase Docker Desktop memory to 4-6GB and rebuild.
