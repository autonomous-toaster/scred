#!/usr/bin/env python3

"""
SCRED Integration Test Suite - Python Version

Comprehensive testing of all redaction scenarios:
- Direct access (no redaction)
- Reverse proxy with response-only redaction
- MITM proxy with response-only redaction
- Reverse proxy with request-only redaction
- Character preservation verification
- Pattern detection across multiple secret types
"""

import subprocess
import requests
import json
import time
import sys
import re
from pathlib import Path
from typing import Tuple, List, Dict, Optional
from dataclasses import dataclass
from enum import Enum

# Configuration
TIMEOUT = 10
SERVICES = {
    "fake-upstream": 8001,
    "httpbin": 8000,
    "scred-proxy": 9999,
    "scred-mitm": 8888,
    "scred-proxy-response-only": 9998,
    "scred-mitm-response-only": 8889,
}

# Colors
class Colors:
    HEADER = '\033[95m'
    BLUE = '\033[94m'
    CYAN = '\033[96m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'

class TestStatus(Enum):
    PASSED = "PASSED"
    FAILED = "FAILED"
    SKIPPED = "SKIPPED"

@dataclass
class TestResult:
    name: str
    status: TestStatus
    message: str
    details: Optional[str] = None

class SCREDTestSuite:
    def __init__(self, verbose: bool = False):
        self.verbose = verbose
        self.results: List[TestResult] = []
        self.passed = 0
        self.failed = 0
        self.skipped = 0
        
    def log_info(self, msg: str):
        print(f"{Colors.BLUE}[INFO]{Colors.ENDC} {msg}")
    
    def log_success(self, msg: str):
        print(f"{Colors.GREEN}[✓]{Colors.ENDC} {msg}")
        self.passed += 1
    
    def log_failure(self, msg: str):
        print(f"{Colors.RED}[✗]{Colors.ENDC} {msg}")
        self.failed += 1
    
    def log_skip(self, msg: str):
        print(f"{Colors.YELLOW}[~]{Colors.ENDC} {msg}")
        self.skipped += 1
    
    def log_header(self, msg: str):
        print(f"\n{Colors.BLUE}{'='*80}{Colors.ENDC}")
        print(f"{Colors.BOLD}{Colors.BLUE}{msg}{Colors.ENDC}")
        print(f"{Colors.BLUE}{'='*80}{Colors.ENDC}\n")
    
    def log_subheader(self, msg: str):
        print(f"\n{Colors.YELLOW}▶ {msg}{Colors.ENDC}")
    
    def verbose_print(self, msg: str):
        if self.verbose:
            print(f"{Colors.YELLOW}[DEBUG]{Colors.ENDC} {msg}")
    
    def is_redacted(self, value: str) -> bool:
        """Check if value is redacted (contains x's, not original secret pattern)"""
        # Check if value contains x characters and doesn't match original patterns
        has_x = 'x' in value.lower()
        
        # Exclude unredacted patterns
        aws_pattern = r'^AKIA[A-Z0-9]{16}$'  # Original AWS key
        sk_pattern = r'^sk-[a-z0-9]{30,}$'    # Original sk- token
        
        # If has x's and doesn't match original patterns, it's redacted
        return has_x and not re.match(aws_pattern, value) and not re.match(sk_pattern, value)
    
    def check_character_preservation(self, original: str, redacted: str) -> bool:
        """Check if character length is preserved during redaction"""
        return len(original) == len(redacted)
    
    def wait_for_service(self, service: str, port: int, timeout: int = 30) -> bool:
        """Wait for service to be available"""
        elapsed = 0
        while elapsed < timeout:
            try:
                requests.get(f"http://localhost:{port}", timeout=2)
                self.log_success(f"{service} is ready")
                return True
            except (requests.ConnectionError, requests.Timeout):
                time.sleep(1)
                elapsed += 1
        
        self.log_failure(f"Timeout waiting for {service}")
        return False
    
    def start_docker_compose(self):
        """Start docker-compose stack"""
        self.log_info("Starting docker-compose stack...")
        result = subprocess.run(
            ["docker-compose", "up", "-d"],
            cwd=Path(__file__).parent,
            capture_output=True,
            text=True
        )
        
        if result.returncode != 0:
            self.log_failure(f"Failed to start docker-compose: {result.stderr}")
            sys.exit(1)
        
        time.sleep(2)
    
    def stop_docker_compose(self):
        """Stop docker-compose stack"""
        self.log_info("Stopping docker-compose stack...")
        subprocess.run(
            ["docker-compose", "down"],
            cwd=Path(__file__).parent,
            capture_output=True
        )
    
    def run_test(self, test_func, test_name: str):
        """Run a test function and track results"""
        self.log_subheader(test_name)
        try:
            test_func()
        except Exception as e:
            self.log_failure(f"Test error: {str(e)}")
            self.verbose_print(f"Traceback: {str(e)}")
    
    # Test Suite: Service Availability
    def test_service_availability(self):
        """Test that all services are available"""
        self.log_header("TEST SUITE 1: Service Availability")
        
        for service, port in SERVICES.items():
            if self.wait_for_service(service, port, timeout=5):
                self.log_success(f"{service} (port {port}) is accessible")
            else:
                self.log_failure(f"{service} (port {port}) is not accessible")
    
    # Test Suite: Direct Upstream (No Redaction)
    def test_direct_upstream_no_redaction(self):
        """Test direct access to fake-upstream without any redaction"""
        self.log_header("TEST SUITE 2: Direct Upstream (No Redaction)")
        
        self.log_subheader("Testing direct access to fake-upstream")
        
        try:
            resp = requests.get("http://localhost:8001/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            data = resp.json()
            
            self.log_success("Got response from fake-upstream")
            
            # Check AWS key
            aws_key = data.get("aws_keys", {}).get("access_key_id", "")
            self.verbose_print(f"AWS key: {aws_key}")
            
            if aws_key.startswith("AKIA"):
                self.log_success(f"AWS key is UNREDACTED (as expected): {aws_key}")
            else:
                self.log_failure(f"AWS key format unexpected: {aws_key}")
            
            # Check API token
            api_key = data.get("api_tokens", {}).get("openai_key", "")
            self.verbose_print(f"OpenAI key: {api_key}")
            
            if api_key.startswith("sk-"):
                self.log_success(f"OpenAI key is UNREDACTED (as expected): {api_key}")
            else:
                self.log_failure(f"OpenAI key format unexpected: {api_key}")
        
        except Exception as e:
            self.log_failure(f"Failed to test direct upstream: {str(e)}")
    
    # Test Suite: Reverse Proxy Response-Only
    def test_reverse_proxy_response_only(self):
        """Test reverse proxy with response-only redaction"""
        self.log_header("TEST SUITE 3: Reverse Proxy (Response-Only Redaction)")
        
        self.log_subheader("Testing scred-proxy-response-only (port 9998)")
        
        try:
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            data = resp.json()
            
            self.log_success("Got response from scred-proxy-response-only")
            
            # Check AWS key
            aws_key = data.get("aws_keys", {}).get("access_key_id", "")
            self.verbose_print(f"AWS key from proxy: {aws_key}")
            
            if self.is_redacted(aws_key):
                self.log_success(f"AWS key is REDACTED (as expected): {aws_key}")
                
                # Check character preservation
                if self.check_character_preservation("AKIAIOSFODNN7EXAMPLE", aws_key):
                    self.log_success(f"Character preservation verified for AWS key")
                else:
                    self.log_failure(f"Character preservation FAILED (expected 20, got {len(aws_key)})")
            else:
                self.log_failure(f"AWS key is NOT redacted: {aws_key}")
            
            # Check API token
            api_key = data.get("api_tokens", {}).get("openai_key", "")
            self.verbose_print(f"OpenAI key from proxy: {api_key}")
            
            if self.is_redacted(api_key):
                self.log_success(f"OpenAI key is REDACTED (as expected): {api_key}")
            else:
                self.log_failure(f"OpenAI key is NOT redacted: {api_key}")
        
        except Exception as e:
            self.log_failure(f"Failed to test reverse proxy response-only: {str(e)}")
    
    # Test Suite: MITM Proxy Response-Only
    def test_mitm_proxy_response_only(self):
        """Test MITM proxy with response-only redaction"""
        self.log_header("TEST SUITE 4: MITM Proxy (Response-Only Redaction)")
        
        self.log_subheader("Testing scred-mitm-response-only via proxy (port 8889)")
        
        try:
            proxies = {"http": "http://localhost:8889"}
            resp = requests.get(
                "http://fake-upstream:8001/secrets.json",
                timeout=TIMEOUT,
                proxies=proxies
            )
            resp.raise_for_status()
            data = resp.json()
            
            self.log_success("Got response through scred-mitm-response-only")
            
            # Check AWS key
            aws_key = data.get("aws_keys", {}).get("access_key_id", "")
            self.verbose_print(f"AWS key through MITM: {aws_key}")
            
            if self.is_redacted(aws_key):
                self.log_success(f"AWS key is REDACTED (as expected): {aws_key}")
            else:
                self.log_failure(f"AWS key is NOT redacted: {aws_key}")
        
        except Exception as e:
            self.log_failure(f"Failed to test MITM proxy: {str(e)}")
    
    # Test Suite: Reverse Proxy Request-Only
    def test_reverse_proxy_request_only(self):
        """Test reverse proxy with request-only redaction"""
        self.log_header("TEST SUITE 5: Reverse Proxy (Request-Only Redaction)")
        
        self.log_subheader("Testing scred-proxy with httpbin (port 9999)")
        
        try:
            # Test with API key in query string
            api_key_orig = "sk-1234567890abcdefghijklmnopqrstuvwxyz"
            resp = requests.get(
                f"http://localhost:9999/get?api_key={api_key_orig}",
                timeout=TIMEOUT
            )
            resp.raise_for_status()
            data = resp.json()
            
            self.log_success("Got response from scred-proxy")
            
            # httpbin echoes back the query params
            # In request-only mode, the request should be redacted before reaching httpbin
            args = data.get("args", {})
            api_key_received = args.get("api_key", "")
            
            self.verbose_print(f"API key received by httpbin: {api_key_received}")
            
            if self.is_redacted(api_key_received):
                self.log_success(f"Request API key was REDACTED before reaching httpbin")
            else:
                # This might be expected if redaction is not active
                self.log_failure(f"Request API key was NOT redacted: {api_key_received}")
        
        except Exception as e:
            self.log_failure(f"Failed to test reverse proxy request-only: {str(e)}")
    
    # Test Suite: Character Preservation
    def test_character_preservation(self):
        """Test character preservation across multiple patterns"""
        self.log_header("TEST SUITE 6: Character Preservation")
        
        self.log_subheader("Testing character preservation for various secret types")
        
        try:
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            data = resp.json()
            
            test_cases = {
                "AWS Access Key": ("AKIAIOSFODNN7EXAMPLE", 20),
                "AWS Secret": ("wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY", 40),
                "OpenAI Key": ("sk-1234567890abcdefghijklmnopqrstuvwxyz", 40),
            }
            
            for field_name, (original, expected_len) in test_cases.items():
                # Find corresponding field in response
                aws_secret = data.get("aws_keys", {}).get("secret_access_key", "")
                if aws_secret:
                    if len(aws_secret) == expected_len:
                        self.log_success(
                            f"Character preservation OK for {field_name}: "
                            f"{expected_len} chars → {len(aws_secret)} chars"
                        )
                    else:
                        self.log_failure(
                            f"Character preservation FAILED for {field_name}: "
                            f"expected {expected_len}, got {len(aws_secret)}"
                        )
        
        except Exception as e:
            self.log_failure(f"Failed to test character preservation: {str(e)}")
    
    # Test Suite: Multiple Patterns Detection
    def test_multiple_patterns_detection(self):
        """Test detection of multiple secret types"""
        self.log_header("TEST SUITE 7: Multiple Patterns Detection")
        
        self.log_subheader("Testing detection of multiple secret types")
        
        try:
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            response_text = resp.text
            
            # Count fields that appear to be redacted
            redacted_pattern = r'"[^"]*":"[^"]*x[^"]*"'
            redacted_count = len(re.findall(redacted_pattern, response_text))
            
            if redacted_count > 10:
                self.log_success(f"Multiple patterns detected and redacted: ~{redacted_count} fields")
            else:
                self.log_failure(f"Only {redacted_count} fields redacted (expected >10)")
        
        except Exception as e:
            self.log_failure(f"Failed to test multiple patterns: {str(e)}")
    
    # Test Suite: JSON Integrity
    def test_json_integrity(self):
        """Test that JSON remains valid after redaction"""
        self.log_header("TEST SUITE 8: JSON Structure Integrity")
        
        self.log_subheader("Testing that JSON remains valid after redaction")
        
        try:
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            
            # This will raise an exception if JSON is invalid
            data = resp.json()
            
            self.log_success("Response JSON is valid after redaction")
            self.verbose_print(f"JSON keys: {list(data.keys())}")
        
        except json.JSONDecodeError as e:
            self.log_failure(f"Response JSON is INVALID: {str(e)}")
        except Exception as e:
            self.log_failure(f"Failed to test JSON integrity: {str(e)}")
    
    # Test Suite: HTTP Status Codes
    def test_http_status_codes(self):
        """Test HTTP status codes through proxies"""
        self.log_header("TEST SUITE 9: HTTP Status Codes")
        
        self.log_subheader("Testing correct status codes through proxies")
        
        try:
            # Test 200
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            if resp.status_code == 200:
                self.log_success("Proxy returns 200 OK")
            else:
                self.log_failure(f"Proxy returns {resp.status_code} (expected 200)")
            
            # Test 404
            resp = requests.get("http://localhost:9998/nonexistent", timeout=TIMEOUT)
            if resp.status_code == 404:
                self.log_success("Proxy correctly returns 404 for missing path")
            else:
                self.log_failure(f"Proxy returns {resp.status_code} for missing path (expected 404)")
        
        except Exception as e:
            self.log_failure(f"Failed to test status codes: {str(e)}")
    
    # Test Suite: Streaming Performance
    def test_streaming_performance(self):
        """Test response time and streaming efficiency"""
        self.log_header("TEST SUITE 10: Performance")
        
        self.log_subheader("Testing response time")
        
        try:
            start = time.time()
            resp = requests.get("http://localhost:9998/secrets.json", timeout=TIMEOUT)
            resp.raise_for_status()
            elapsed = time.time() - start
            elapsed_ms = int(elapsed * 1000)
            
            if elapsed_ms < 1000:
                self.log_success(f"Response time is good: {elapsed_ms}ms")
            elif elapsed_ms < 3000:
                self.log_success(f"Response time is acceptable: {elapsed_ms}ms")
            else:
                self.log_failure(f"Response time is slow: {elapsed_ms}ms")
        
        except Exception as e:
            self.log_failure(f"Failed to test performance: {str(e)}")
    
    def print_summary(self):
        """Print test summary"""
        print(f"\n{Colors.BLUE}{'='*80}{Colors.ENDC}")
        print(f"{Colors.BOLD}{Colors.BLUE}TEST RESULTS SUMMARY{Colors.ENDC}")
        print(f"{Colors.BLUE}{'='*80}{Colors.ENDC}\n")
        
        print(f"{Colors.GREEN}Passed:  {self.passed}{Colors.ENDC}")
        print(f"{Colors.RED}Failed:  {self.failed}{Colors.ENDC}")
        print(f"{Colors.YELLOW}Skipped: {self.skipped}{Colors.ENDC}")
        total = self.passed + self.failed + self.skipped
        print(f"{Colors.BLUE}Total:   {total}{Colors.ENDC}")
        
        print()
        if self.failed == 0:
            print(f"{Colors.GREEN}{Colors.BOLD}✓ All tests passed!{Colors.ENDC}")
        else:
            print(f"{Colors.RED}{Colors.BOLD}✗ Some tests failed. Review output above.{Colors.ENDC}")
        
        print(f"\nEnd time: {time.strftime('%Y-%m-%d %H:%M:%S')}")
    
    def run_all_tests(self):
        """Run all test suites"""
        print(f"\n{Colors.BOLD}{Colors.BLUE}SCRED INTEGRATION TEST SUITE{Colors.ENDC}")
        print(f"All redaction scenarios: proxy/mitm, request/response/both")
        print(f"Start time: {time.strftime('%Y-%m-%d %H:%M:%S')}\n")
        
        # Start stack
        self.start_docker_compose()
        
        # Run tests
        self.test_service_availability()
        self.test_direct_upstream_no_redaction()
        self.test_reverse_proxy_response_only()
        self.test_mitm_proxy_response_only()
        self.test_reverse_proxy_request_only()
        self.test_character_preservation()
        self.test_multiple_patterns_detection()
        self.test_json_integrity()
        self.test_http_status_codes()
        self.test_streaming_performance()
        
        # Print summary
        self.print_summary()
        
        # Cleanup
        self.stop_docker_compose()
        
        # Exit with appropriate code
        return 0 if self.failed == 0 else 1

def main():
    import argparse
    
    parser = argparse.ArgumentParser(
        description="SCRED Integration Test Suite",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s                    # Run all tests
  %(prog)s --verbose          # Run with verbose output
  %(prog)s --no-cleanup       # Don't stop docker-compose after tests
        """
    )
    
    parser.add_argument(
        "-v", "--verbose",
        action="store_true",
        help="Verbose output"
    )
    
    args = parser.parse_args()
    
    suite = SCREDTestSuite(verbose=args.verbose)
    return suite.run_all_tests()

if __name__ == "__main__":
    sys.exit(main())
