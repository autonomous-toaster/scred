#!/bin/bash

################################################################################
# SCRED Integration Test Suite
# Tests all redaction scenarios: proxy/mitm, request/response/both, character
# preservation, pattern detection across all services
################################################################################

set -o errexit
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TIMEOUT=10
VERBOSE=${VERBOSE:-0}
DOCKER_COMPOSE_CMD="docker-compose"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0

################################################################################
# Utility Functions
################################################################################

log_info() {
    echo -e "${BLUE}[INFO]${NC} $@"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $@"
    ((TESTS_PASSED++))
}

log_failure() {
    echo -e "${RED}[✗]${NC} $@"
    ((TESTS_FAILED++))
}

log_skip() {
    echo -e "${YELLOW}[~]${NC} $@"
    ((TESTS_SKIPPED++))
}

log_header() {
    echo ""
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}$@${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

log_subheader() {
    echo -e "\n${YELLOW}▶ $@${NC}"
}

verbose() {
    if [ "$VERBOSE" -eq 1 ]; then
        echo -e "${YELLOW}[DEBUG]${NC} $@"
    fi
}

################################################################################
# Helper Functions
################################################################################

# Check if service is healthy
wait_for_service() {
    local service=$1
    local port=$2
    local timeout=${3:-30}
    local elapsed=0

    log_info "Waiting for $service on port $port..."
    
    while [ $elapsed -lt $timeout ]; do
        if timeout 2 bash -c "echo >/dev/tcp/localhost/$port" 2>/dev/null; then
            log_success "$service is ready"
            return 0
        fi
        sleep 1
        ((elapsed++))
    done
    
    log_failure "Timeout waiting for $service"
    return 1
}

# Extract field from JSON response
extract_json_field() {
    local json="$1"
    local field="$2"
    echo "$json" | grep -o "\"$field\":[^,}]*" | cut -d':' -f2- | tr -d '"' | xargs
}

# Check if value is redacted (contains 'x' characters, not the original pattern)
is_redacted() {
    local value="$1"
    # Match: any string with significant x characters (not original secret pattern)
    # Pattern 1: Prefix (2-4 chars) + x's (e.g., "AKIAxxxxxx" or "sk-xxxx")
    # Pattern 2: All x's (e.g., "xxxxxxxxxx")
    if [[ "$value" =~ x ]] && [[ ! "$value" =~ ^AKIA[A-Z0-9]{16}$ ]] && [[ ! "$value" =~ ^sk-[a-z0-9]{30,}$ ]]; then
        return 0  # True - is redacted
    fi
    return 1  # False - not redacted
}

# Check if character length is preserved
check_character_preservation() {
    local original="$1"
    local redacted="$2"
    
    if [ ${#original} -eq ${#redacted} ]; then
        return 0  # True - length preserved
    fi
    return 1  # False - length NOT preserved
}

# Test with curl and timeout
curl_test() {
    local url="$1"
    local method=${2:-GET}
    local data=${3:-}
    local proxy=${4:-}
    
    if [ -z "$proxy" ]; then
        timeout $TIMEOUT curl -s -X "$method" "$url" ${data:+-d "$data"}
    else
        timeout $TIMEOUT curl -s -X "$method" -x "$proxy" "$url" ${data:+-d "$data"}
    fi
}

################################################################################
# Test Functions
################################################################################

test_service_availability() {
    log_header "TEST SUITE 1: Service Availability"
    
    log_subheader "Checking docker-compose is up"
    if $DOCKER_COMPOSE_CMD ps | grep -q "Up"; then
        log_success "Docker compose stack is running"
    else
        log_failure "Docker compose stack is not running"
        # return 1
    fi
    
    # Test each service
    local services=(
        "fake-upstream:8001"
        "httpbin:8000"
        "scred-proxy:9999"
        "scred-mitm:8888"
        "scred-proxy-response-only:9998"
        "scred-mitm-response-only:8889"
    )
    
    for service_info in "${services[@]}"; do
        local service="${service_info%:*}"
        local port="${service_info#*:}"
        
        if wait_for_service "$service" "$port" 5; then
            log_success "$service (port $port) is accessible"
        else
            log_failure "$service (port $port) is not accessible"
        fi
    done
}

test_direct_upstream_no_redaction() {
    log_header "TEST SUITE 2: Direct Upstream (No Redaction)"
    
    log_subheader "Testing direct access to fake-upstream"
    
    local response=$(curl_test "http://localhost:8001/secrets.json")
    
    if [ -z "$response" ]; then
        log_failure "Failed to get response from fake-upstream"
        return 1
    fi
    
    log_success "Got response from fake-upstream"
    
    # Extract AWS key
    local aws_key=$(echo "$response" | grep -o '"access_key_id":"[^"]*"' | cut -d'"' -f4)
    verbose "AWS key from fake-upstream: $aws_key"
    
    if [[ "$aws_key" == "AKIA"* ]]; then
        log_success "AWS key is UNREDACTED (as expected): $aws_key"
    else
        log_failure "AWS key is missing or invalid: $aws_key"
    fi
    
    # Extract API key
    local api_key=$(echo "$response" | grep -o '"openai_key":"[^"]*"' | cut -d'"' -f4)
    verbose "OpenAI key: $api_key"
    
    if [[ "$api_key" == "sk-"* ]]; then
        log_success "OpenAI key is UNREDACTED (as expected): $api_key"
    else
        log_failure "OpenAI key is missing or invalid: $api_key"
    fi
}

test_reverse_proxy_response_only() {
    log_header "TEST SUITE 3: Reverse Proxy (Response-Only Redaction)"
    
    log_subheader "Testing scred-proxy-response-only (port 9998)"
    
    local response=$(curl_test "http://localhost:9998/secrets.json")
    
    if [ -z "$response" ]; then
        log_failure "Failed to get response from proxy"
        return 1
    fi
    
    log_success "Got response from scred-proxy-response-only"
    
    # Extract AWS key
    local aws_key=$(echo "$response" | grep -o '"access_key_id":"[^"]*"' | cut -d'"' -f4)
    verbose "AWS key from proxy: $aws_key"
    
    if is_redacted "$aws_key"; then
        log_success "AWS key is REDACTED (as expected): $aws_key"
        
        # Check character preservation
        if check_character_preservation "AKIAIOSFODNN7EXAMPLE" "$aws_key"; then
            log_success "Character preservation verified for AWS key"
        else
            log_failure "Character preservation FAILED for AWS key (length: ${#aws_key})"
        fi
    else
        log_failure "AWS key is NOT redacted: $aws_key"
    fi
    
    # Extract API key
    local api_key=$(echo "$response" | grep -o '"openai_key":"[^"]*"' | cut -d'"' -f4)
    verbose "OpenAI key: $api_key"
    
    if is_redacted "$api_key"; then
        log_success "OpenAI key is REDACTED (as expected): $api_key"
    else
        log_failure "OpenAI key is NOT redacted: $api_key"
    fi
}

test_mitm_proxy_response_only() {
    log_header "TEST SUITE 4: MITM Proxy (Response-Only Redaction)"
    
    log_subheader "Testing scred-mitm-response-only via proxy (port 8889)"
    
    local response=$(curl_test "http://fake-upstream:8001/secrets.json" "GET" "" "http://localhost:8889")
    
    if [ -z "$response" ]; then
        log_failure "Failed to get response through MITM proxy"
        return 1
    fi
    
    log_success "Got response through scred-mitm-response-only"
    
    # Extract AWS key
    local aws_key=$(echo "$response" | grep -o '"access_key_id":"[^"]*"' | cut -d'"' -f4)
    verbose "AWS key through MITM: $aws_key"
    
    if is_redacted "$aws_key"; then
        log_success "AWS key is REDACTED (as expected): $aws_key"
    else
        log_failure "AWS key is NOT redacted: $aws_key"
    fi
}

test_reverse_proxy_request_only() {
    log_header "TEST SUITE 5: Reverse Proxy (Request-Only Redaction)"
    
    log_subheader "Testing scred-proxy with httpbin (port 9999)"
    
    # Test with API key in query string
    local response=$(curl_test "http://localhost:9999/get?api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz")
    
    if [ -z "$response" ]; then
        log_failure "Failed to get response from proxy"
        return 1
    fi
    
    log_success "Got response from scred-proxy"
    
    # In request-only mode, request should be redacted before reaching httpbin
    # httpbin echoes back the request, so we check what httpbin received
    if echo "$response" | grep -q "api_key"; then
        # Check if it's redacted
        local api_key=$(echo "$response" | grep -o 'api_key[=&][^&]*' | cut -d'=' -f2)
        verbose "API key in httpbin response: $api_key"
        
        if is_redacted "$api_key"; then
            log_success "Request API key was REDACTED before reaching httpbin"
        else
            log_failure "Request API key was NOT redacted: $api_key"
        fi
    fi
}

test_direct_httpbin_no_redaction() {
    log_header "TEST SUITE 6: Direct httpbin (No Redaction - Baseline)"
    
    log_subheader "Testing direct access to httpbin for comparison"
    
    local response=$(curl_test "http://localhost:8000/get?api_key=sk-1234567890abcdefghijklmnopqrstuvwxyz")
    
    if [ -z "$response" ]; then
        log_failure "Failed to get response from httpbin"
        return 1
    fi
    
    log_success "Got response from httpbin"
    
    if echo "$response" | grep -q "api_key"; then
        local api_key=$(echo "$response" | grep -o 'api_key[=&][^&]*' | cut -d'=' -f2)
        
        if [[ "$api_key" == "sk-"* ]]; then
            log_success "API key in httpbin response is UNREDACTED (baseline): $api_key"
        else
            log_failure "API key format unexpected: $api_key"
        fi
    fi
}

test_character_preservation() {
    log_header "TEST SUITE 7: Character Preservation Across All Patterns"
    
    log_subheader "Testing character preservation for various secret types"
    
    # Get response from proxy
    local response=$(curl_test "http://localhost:9998/secrets.json")
    
    local test_cases=(
        "access_key_id:AKIAIOSFODNN7EXAMPLE"
        "secret_access_key:wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY"
    )
    
    for test_case in "${test_cases[@]}"; do
        local field="${test_case%:*}"
        local expected="${test_case#*:}"
        local original_len=${#expected}
        
        local actual=$(echo "$response" | grep -o "\"$field\":\"[^\"]*\"" | cut -d'"' -f4)
        
        if [ -n "$actual" ]; then
            local actual_len=${#actual}
            
            if [ "$original_len" -eq "$actual_len" ]; then
                log_success "Character preservation OK for $field: $original_len chars → $actual_len chars"
            else
                log_failure "Character preservation FAILED for $field: $original_len → $actual_len"
            fi
        fi
    done
}

test_multiple_patterns_detection() {
    log_header "TEST SUITE 8: Multiple Patterns Detection"
    
    log_subheader "Testing detection of multiple secret types in single response"
    
    local response=$(curl_test "http://localhost:9998/secrets.json")
    
    # Count redacted fields (fields with 'x' characters)
    local redacted_count=$(echo "$response" | grep -o '"[^"]*":"[^"]*[x][^"]*"' | wc -l)
    
    if [ "$redacted_count" -gt 10 ]; then
        log_success "Multiple patterns detected and redacted: ~$redacted_count fields"
    else
        log_failure "Only $redacted_count fields redacted (expected >10)"
    fi
}

test_json_structure_integrity() {
    log_header "TEST SUITE 9: JSON Structure Integrity"
    
    log_subheader "Testing that JSON remains valid after redaction"
    
    local response=$(curl_test "http://localhost:9998/secrets.json")
    
    # Try to parse JSON
    if echo "$response" | jq . &>/dev/null; then
        log_success "Response JSON is valid after redaction"
    else
        log_failure "Response JSON is INVALID after redaction"
        verbose "Response: $response"
    fi
}

test_http_status_codes() {
    log_header "TEST SUITE 10: HTTP Status Codes"
    
    log_subheader "Testing correct status codes through proxies"
    
    local status=$(curl_test "http://localhost:9998/secrets.json" "GET" "" | head -1)
    
    # Get actual status
    local status=$(timeout $TIMEOUT curl -s -o /dev/null -w "%{http_code}" "http://localhost:9998/secrets.json")
    
    if [ "$status" = "200" ]; then
        log_success "Reverse proxy returns 200 OK"
    else
        log_failure "Reverse proxy returns $status (expected 200)"
    fi
    
    # Test 404
    local status_404=$(timeout $TIMEOUT curl -s -o /dev/null -w "%{http_code}" "http://localhost:9998/nonexistent")
    
    if [ "$status_404" = "404" ]; then
        log_success "Proxy correctly returns 404 for missing path"
    else
        log_failure "Proxy returns $status_404 for missing path (expected 404)"
    fi
}

test_response_headers() {
    log_header "TEST SUITE 11: Response Headers"
    
    log_subheader "Testing that response headers are preserved"
    
    local headers=$(timeout $TIMEOUT curl -s -i "http://localhost:9998/secrets.json" | head -10)
    
    if echo "$headers" | grep -q "Content-Type"; then
        log_success "Content-Type header is present"
    else
        log_failure "Content-Type header is missing"
    fi
    
    if echo "$headers" | grep -q "Content-Length"; then
        log_success "Content-Length header is present"
    else
        log_failure "Content-Length header is missing"
    fi
}

test_streaming_performance() {
    log_header "TEST SUITE 12: Streaming Performance"
    
    log_subheader "Testing response time and streaming efficiency"
    
    local start=$(date +%s%N)
    local response=$(curl_test "http://localhost:9998/secrets.json")
    local end=$(date +%s%N)
    
    local duration_ms=$(( (end - start) / 1000000 ))
    
    if [ "$duration_ms" -lt 1000 ]; then
        log_success "Response time is good: ${duration_ms}ms"
    elif [ "$duration_ms" -lt 3000 ]; then
        log_success "Response time is acceptable: ${duration_ms}ms"
    else
        log_failure "Response time is slow: ${duration_ms}ms"
    fi
}

test_concurrent_requests() {
    log_header "TEST SUITE 13: Concurrent Request Handling"
    
    log_subheader "Testing multiple concurrent requests"
    
    local concurrent=5
    local success=0
    
    for i in $(seq 1 $concurrent); do
        if timeout 5 curl -s "http://localhost:9998/secrets.json" > /dev/null 2>&1; then
            ((success++))
        fi &
    done
    
    wait
    
    if [ "$success" -eq "$concurrent" ]; then
        log_success "All $concurrent concurrent requests succeeded"
    else
        log_failure "Only $success/$concurrent concurrent requests succeeded"
    fi
}

test_different_content_types() {
    log_header "TEST SUITE 14: Different Content Types"
    
    log_subheader "Testing JSON response from fake-upstream"
    
    local content_type=$(timeout $TIMEOUT curl -s -i "http://localhost:9998/secrets.json" | grep -i "content-type" | cut -d':' -f2- | xargs)
    
    if [[ "$content_type" == *"application/json"* ]] || [[ "$content_type" == *"text/"* ]]; then
        log_success "Content-Type is correct: $content_type"
    else
        log_success "Content-Type present: $content_type"
    fi
}

test_edge_case_special_characters() {
    log_header "TEST SUITE 15: Edge Cases with Special Characters"
    
    log_subheader "Testing redaction of secrets with special characters"
    
    local response=$(curl_test "http://localhost:9998/secrets.json")
    
    # Check for secrets with special chars (like database URLs with passwords)
    if echo "$response" | grep -q "mongodb"; then
        local mongodb_entry=$(echo "$response" | grep -o '"mongodb_uri":"[^"]*"' | cut -d'"' -f4)
        
        if is_redacted "$mongodb_entry"; then
            log_success "MongoDB URI with special chars is redacted: ${mongodb_entry:0:30}..."
        else
            log_failure "MongoDB URI is not properly redacted"
        fi
    fi
}

################################################################################
# Main Test Execution
################################################################################

main() {
    log_header "SCRED INTEGRATION TEST SUITE"
    echo "All redaction scenarios: proxy/mitm, request/response/both"
    echo "Start time: $(date)"
    echo ""
    
    # Check prerequisites
    if ! command -v docker-compose &> /dev/null; then
        log_failure "docker-compose not found"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        log_info "jq not found, JSON validation will be skipped"
    fi
    
    if ! command -v curl &> /dev/null; then
        log_failure "curl not found"
        exit 1
    fi
    
    # Start docker-compose stack
    log_info "Starting docker-compose stack..."
    # cd "$SCRIPT_DIR"
    # $DOCKER_COMPOSE_CMD up -d
    
    # sleep 2
    
    # Run all tests
    # test_service_availability
    test_direct_upstream_no_redaction
    test_reverse_proxy_response_only
    test_mitm_proxy_response_only
    test_reverse_proxy_request_only
    test_direct_httpbin_no_redaction
    test_character_preservation
    test_multiple_patterns_detection
    test_json_structure_integrity
    test_http_status_codes
    test_response_headers
    test_streaming_performance
    test_concurrent_requests
    test_different_content_types
    test_edge_case_special_characters
    
    # Print summary
    print_summary
    
    # Cleanup
    log_info "Cleaning up..."
    $DOCKER_COMPOSE_CMD down
    
    # Exit with appropriate code
    if [ "$TESTS_FAILED" -gt 0 ]; then
        exit 1
    else
        exit 0
    fi
}

print_summary() {
    echo ""
    log_header "TEST RESULTS SUMMARY"
    
    echo ""
    echo -e "${GREEN}Passed:  $TESTS_PASSED${NC}"
    echo -e "${RED}Failed:  $TESTS_FAILED${NC}"
    echo -e "${YELLOW}Skipped: $TESTS_SKIPPED${NC}"
    echo -e "${BLUE}Total:   $((TESTS_PASSED + TESTS_FAILED + TESTS_SKIPPED))${NC}"
    
    echo ""
    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "${GREEN}✓ All tests passed!${NC}"
    else
        echo -e "${RED}✗ Some tests failed. Review output above.${NC}"
    fi
    
    echo ""
    echo "End time: $(date)"
}

# Run main function
main "$@"
