#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}SCRED Docker Build & Test Script${NC}"
echo "========================================"

# Check if docker is running
if ! docker info &>/dev/null; then
    echo -e "${RED}❌ Docker is not running. Please start Docker Desktop.${NC}"
    exit 1
fi

# Set working directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$REPO_ROOT"

echo -e "${YELLOW}Step 1: Preparing build context...${NC}"
echo "  Repository root: $REPO_ROOT"
echo "  Docker-compose dir: $SCRIPT_DIR"
echo ""

# Check for .dockerignore
if [ -f "$REPO_ROOT/.dockerignore" ]; then
    echo -e "${GREEN}✓ .dockerignore found${NC}"
else
    echo -e "${YELLOW}⚠ .dockerignore not found, build context may be large${NC}"
fi

echo ""
echo -e "${YELLOW}Step 2: Cleaning up old containers...${NC}"
docker-compose -f "$SCRIPT_DIR/docker-compose.yml" down 2>/dev/null || true
echo -e "${GREEN}✓ Cleaned up old containers${NC}"

echo ""
echo -e "${YELLOW}Step 3: Building Docker images...${NC}"
echo "  This may take 5-10 minutes on first build"
echo "  Building with -j 1 to minimize memory usage"
echo ""

# Build with better error handling
if docker-compose \
    -f "$SCRIPT_DIR/docker-compose.yml" \
    build \
    --progress=plain \
    --build-arg BUILDKIT_INLINE_CACHE=1 \
    2>&1; then
    echo -e "${GREEN}✓ Docker images built successfully${NC}"
else
    echo -e "${RED}❌ Docker build failed${NC}"
    echo ""
    echo "Troubleshooting steps:"
    echo "1. Check Docker Desktop memory: Set to 4GB+ in settings"
    echo "2. Clean Docker cache: docker system prune -a"
    echo "3. Free disk space: Ensure 10GB+ available"
    echo "4. Check Docker logs: docker-compose logs"
    exit 1
fi

echo ""
echo -e "${YELLOW}Step 4: Starting docker-compose stack...${NC}"

# Start with better timeout and health check
if docker-compose -f "$SCRIPT_DIR/docker-compose.yml" up -d --wait 2>&1; then
    echo -e "${GREEN}✓ Docker stack started successfully${NC}"
else
    echo -e "${RED}❌ Docker stack failed to start${NC}"
    docker-compose -f "$SCRIPT_DIR/docker-compose.yml" logs
    exit 1
fi

echo ""
echo -e "${YELLOW}Step 5: Waiting for services to be healthy...${NC}"
sleep 5

# Check service status
echo ""
echo "Service status:"
docker-compose -f "$SCRIPT_DIR/docker-compose.yml" ps

echo ""
echo -e "${YELLOW}Step 6: Running quick test...${NC}"
echo "  Testing fake-upstream (8001)..."
if curl -s http://localhost:8001/secrets.json >/dev/null 2>&1; then
    echo -e "${GREEN}  ✓ fake-upstream responding${NC}"
else
    echo -e "${RED}  ✗ fake-upstream not responding${NC}"
fi

echo "  Testing httpbin (8000)..."
if curl -s http://localhost:8000/get >/dev/null 2>&1; then
    echo -e "${GREEN}  ✓ httpbin responding${NC}"
else
    echo -e "${RED}  ✗ httpbin not responding${NC}"
fi

echo ""
echo -e "${GREEN}✓ Docker build and startup complete!${NC}"
echo ""
echo "Available services:"
echo "  - fake-upstream: http://localhost:8001"
echo "  - httpbin: http://localhost:8000"
echo "  - scred-proxy (request-only): http://localhost:9999"
echo "  - scred-mitm (request-only): http://localhost:8888"
echo "  - scred-proxy-response-only: http://localhost:9998"
echo "  - scred-mitm-response-only: http://localhost:8889"
echo ""
echo "Run integration tests:"
echo "  cd docker-compose && ./test-all-scenarios.sh"
echo ""
echo "Stop the stack:"
echo "  docker-compose -f $SCRIPT_DIR/docker-compose.yml down"
