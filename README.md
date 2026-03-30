# SCRED - Secret Redaction Engine

🔒 **Fast, safe, and comprehensive secret redaction for logs, configs, and environment variables.**

Detects and redacts 372+ sensitive credential patterns with character-preserving redaction and streaming support.

## What It Does

Finds and redacts secrets while preserving structure and length:

```bash
# Redact all secrets from stdin (default: redacts ALL patterns)
$ cat secrets.txt | scred
$ env | scred > redacted_env.txt

# Redact only critical secrets
$ scred --redact CRITICAL < input.txt

# Show what was detected (debug mode)
$ scred --detect-only < input.txt

# Stream large files with bounded memory
$ scred < 1GB_logfile.txt > redacted_logfile.txt
```

**Key Features**:
- **372+ credential patterns** - AWS, GitHub, Stripe, JWT, SSH keys, databases, etc.
- **Redacts ALL by default** - Maximum security out of the box
- **Character-preserving** - Output same length as input (keeps first 4 chars of tokens)
- **Streaming mode** - <64KB memory, 102+ MB/s throughput
- **Pattern tiers** - CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS
- **Zero-regex** - Fast, no regex dependency, <100µs detection per 10KB
- **100% safe Rust** - No unsafe code

## Quick Start

### Installation

```bash
# Clone and build
git clone https://github.com/autonomous-toaster/scred.git
cd scred
cargo build --release

# Binary at: ./target/release/scred
./target/release/scred --help
```

### Basic Usage

```bash
# Redact all secrets from stdin (default behavior)
cat secrets.txt | scred
env | scred > redacted_env.txt

# Redact only CRITICAL patterns
scred --redact CRITICAL < input.txt

# Redact only API_KEYS
scred --redact API_KEYS < input.txt

# Show what was detected (debug mode)
scred --detect-only < input.txt

# Verbose output with statistics
scred -v < input.txt
```

### Advanced Pattern Selection

```bash
# Redact specific providers using glob patterns
scred --redact "mysql*,postgresql*,mongodb*" input.txt
scred --redact "aws-*,gcp-*,azure-*" input.txt
scred --redact "openai*,anthropic*,huggingface*" input.txt

# Combine tiers + patterns
scred --redact "CRITICAL,aws-*,github-*" input.txt

# Exclude patterns
scred --redact "CRITICAL,API_KEYS,!test-*,!mock-*" input.txt

# Complex filtering
scred --redact "CRITICAL,aws-*,gcp-*,mysql*,postgresql*" input.txt
```

### Pattern Syntax

- `TIER_NAME` - Redact entire tier (CRITICAL, API_KEYS, INFRASTRUCTURE, SERVICES, PATTERNS)
- `pattern*` - Glob pattern (mysql*, aws-*, openai*)
- `pattern?` - Single char wildcard (aws-?)
- `exact-name` - Exact pattern match
- `!pattern` - Exclude pattern
- `ALL` - All 372 patterns


## What Gets Redacted

### By Tier (372 patterns total)

**CRITICAL** (87 patterns) - Highest priority
- AWS (AKIA, secret keys, session tokens, MFA)
- Azure (AD client secrets, connection strings)
- GCP (private keys, service accounts)
- GitHub (PAT, OAuth, app tokens)
- Stripe (API keys, payment intents)
- OpenAI, Anthropic, Claude API keys
- MongoDB, PostgreSQL, MySQL connection strings
- Vault, Consul tokens
- JWT tokens (eyJ prefix)

**API_KEYS** (60+ patterns)
- OpenAI, Anthropic, Google, Slack, Twilio
- SendGrid, Mailgun, Discord, Telegram
- Notion, Hugging Face, Linear, Vercel
- Supabase, Heroku, npm, PyPI
- And 40+ more providers

**INFRASTRUCTURE** (124 patterns)
- Kubernetes secrets, Docker registry
- Terraform, CloudFormation credentials
- SSH keys, PGP keys, certificates
- Helm, Ansible, SaltStack secrets
- Databricks, Grafana, Prometheus tokens
- Vault, Consul, etcd credentials

**SERVICES** (22 patterns)
- Payment processors (Razorpay, Square, Braintree)
- Communication services (Twilio, SendGrid)
- SaaS credentials and webhooks

**PATTERNS** (79 patterns)
- Bearer tokens, BasicAuth headers
- Private keys, certificates
- Database URLs, connection strings
- Generic API key patterns

### Supported Providers (250+)

**Cloud**: AWS, Azure, GCP, DigitalOcean, Linode, Vultr  
**Databases**: MongoDB, PostgreSQL, MySQL, Redis, Cassandra, Elasticsearch  
**APIs**: OpenAI, Anthropic, Stripe, Slack, Discord, Twilio, SendGrid  
**DevOps**: Kubernetes, Docker, Terraform, Ansible, Helm, Chef  
**Git**: GitHub, GitLab, Gitea, Bitbucket  
**Secrets**: Vault, Consul, 1Password, LastPass  
**And 200+ more...**

See `scred --list-patterns` for complete list.

## Architecture

### Detection Pipeline

Zero-regex architecture using optimized prefix matching:

```
Input Text
    ↓
Simple Prefix Matching (24 patterns, <1µs per character)
    ↓ (no match)
Prefix + Charset Validation (221 patterns, <2µs per character)
    ↓ (no match)
JWT Pattern Detection (eyJ prefix + 2 dots, O(1) check)
    ↓ (no match)
Multi-line Pattern Detection (SSH keys, certificates, ~30 patterns)
    ↓
Character-Preserving Redaction (position-based)
    ↓
Output (identical length as input)
```

### Performance Characteristics

- **Detection**: <100µs per 10KB payload
- **Redaction**: <100ms per 1MB payload
- **Memory**: <64KB typical (streaming mode, bounded buffer)
- **Latency**: Sub-millisecond for small inputs
- **Throughput**: 102-116 MB/s (full streaming pipeline)
- **Zero-copy**: Reuses detected match data for redaction

## Building from Source

### Release Build
```bash
git clone https://github.com/your-org/scred.git
cd scred

# Build
cargo build --release

# Test (71 tests)
cargo test --lib

# Run
./target/release/scred --help
```

### Development
```bash
# Build with debug symbols
cargo build

# Run tests with output
cargo test --lib -- --nocapture

# Run specific test
cargo test pattern_selection -- --nocapture
```

## Testing

### Run All Tests
```bash
# 71 tests covering all features
cargo test --lib

# Run with output
cargo test --lib -- --nocapture

# Run specific suite
cargo test pattern_selection
cargo test detector
cargo test redactor
```

### Performance Testing
```bash
# Build release binary
cargo build --release

# Test throughput
echo "test content..." | ./target/release/scred

# Profile with real workloads
./target/release/scred < large_logfile.txt > redacted.txt
```

## Documentation

For architecture decisions and development notes, see the git history and inline code comments.

## CLI Reference

### Command Line Options

```bash
scred [OPTIONS] [FILE]

Arguments:
  FILE                        File to redact (stdin if not provided)

Pattern Options:
  --detect <TYPES>            Patterns to detect (default: ALL)
  --redact <TYPES>            Patterns to redact (default: ALL)

Mode Options:
  -v, --verbose               Show statistics and detected patterns
  --env-mode, --env           Force environment variable mode
  --text-mode                 Force text/pattern mode
  --detect-only               Show detection result and exit (debug)

Information:
  --list-patterns             Show all 372 patterns
  --describe <NAME>           Show details for a specific pattern
  --filter-type <TYPE>        Filter patterns: fast, structured, regex
  --help                      Show help
  --version                   Show version
```

### Pattern Tiers

Available tiers for `--detect` and `--redact`:
- **CRITICAL** (87 patterns): AWS, Azure, GCP, GitHub, Stripe, OpenAI, JWT, databases
- **API_KEYS** (60+ patterns): OpenAI, Anthropic, Google, Slack, Twilio, Discord, etc.
- **INFRASTRUCTURE** (124 patterns): Kubernetes, Docker, SSH keys, Terraform, Vault, etc.
- **SERVICES** (22 patterns): Payment processors, SaaS, webhooks, service accounts
- **PATTERNS** (79 patterns): Bearer tokens, BasicAuth, private keys, connection strings
- **ALL** (372 total): All patterns (default)

### As Library

```rust
use scred::detect_all;

let text = "AWS key: AKIAIOSFODNN7EXAMPLE";
let matches = detect_all(text.as_bytes());

for m in matches.iter() {
    println!("Found at {}-{}", m.start, m.end);
}
```


**Key optimizations**:
- In-memory buffering for small inputs
- Streaming (frame ring buffer) for large inputs
- Zero-copy redaction using position-based matching
- Pattern-aware tier filtering

## Available Patterns

### Pattern Selection by Tier

SCRED organizes 273+ patterns into 5 risk tiers:

**CRITICAL** (24 patterns):
AWS credentials, GitHub tokens, Stripe keys, database credentials
```bash
scred --redact CRITICAL input.txt
```

**API_KEYS** (60+ patterns):
OpenAI, Anthropic, Twilio, SendGrid, Slack, Discord, HuggingFace
```bash
scred --redact API_KEYS input.txt
```

**INFRASTRUCTURE** (40+ patterns):
Kubernetes, Docker, Vault, Grafana, DataDog, New Relic
```bash
scred --redact INFRASTRUCTURE input.txt
```

**SERVICES** (100+ patterns):
Payment processors, communication services, analytics
```bash
scred --redact SERVICES input.txt
```

**PATTERNS** (50+ patterns):
JWT tokens, Bearer tokens, Basic Auth, generic credentials
```bash
scred --redact PATTERNS input.txt
```

### Pattern Selection by Name (Glob Matching)

Use wildcards to select specific pattern families:

**Database patterns**:
```bash
scred --redact "mysql*,postgresql*,mongodb*,redis*,mariadb*" input.txt
```

**Cloud providers**:
```bash
scred --redact "aws-*,gcp-*,azure-*,digitalocean-*" input.txt
```

**AI/ML APIs**:
```bash
scred --redact "openai*,anthropic*,huggingface*,cohere*" input.txt
```

**Payment processors**:
```bash
scred --redact "stripe*,paypal*,square*,braintree*" input.txt
```

### List All Available Patterns

```bash
# Show all 273+ pattern names
scred --list-patterns

# Filter by provider
scred --list-patterns | grep mysql
scred --list-patterns | grep aws
scred --list-patterns | grep github
```

### Common Pattern Examples

**Individual patterns**:
- `aws-akia` - AWS Access Key ID
- `github-pat` - GitHub Personal Access Token
- `openai-api-key` - OpenAI API Key
- `stripe-sk-live` - Stripe Secret Key (live)
- `mysql-password` - MySQL Connection Password
- `postgresql-dsn` - PostgreSQL Data Source Name

**Pattern families** (use wildcards):
- `aws-*` - All AWS patterns (akia, secret-access-key, etc.)
- `github-*` - All GitHub patterns (pat, oauth, refresh, etc.)
- `mysql*` - All MySQL patterns (mysql-password, mysql-dsn, etc.)
- `*-password` - All -password patterns (mysql-password, postgres-password, etc.)
- `stripe-*` - All Stripe patterns (sk-live, sk-test, webhook, etc.)


## Tips & Common Use Cases

### Environment-Specific Pattern Selection

**Development** (catch everything):
```bash
export SCRED_REDACT_PATTERNS="CRITICAL,API_KEYS,INFRASTRUCTURE,SERVICES,PATTERNS"
scred < logfile.txt
```

**Staging** (CRITICAL + common databases):
```bash
export SCRED_REDACT_PATTERNS="CRITICAL,API_KEYS,mysql*,postgresql*,mongodb*"
scred < logfile.txt
```

**Production** (CRITICAL only, exclude test patterns):
```bash
export SCRED_REDACT_PATTERNS="CRITICAL,!test-*,!example-*,!sandbox-*"
scred < logfile.txt
```

### Microservices Architecture

**Database layer only**:
```bash
scred --redact "mysql*,postgresql*,mongodb*,redis*" < database_logs.txt
```

**API layer only**:
```bash
scred --redact "openai*,anthropic*,stripe*,github-*" < api_logs.txt
```

**Full stack**:
```bash
scred --redact "CRITICAL,API_KEYS,mysql*,postgresql*,mongodb*,redis*,openai*,github-*" < app.log
```

### Real-World Examples

**Scrub AWS logs**:
```bash
scred --redact "aws-*,CRITICAL" < cloudtrail.log > cleaned.log
```

**Protect GitHub CI logs**:
```bash
scred --redact "github-*,openai*,CRITICAL" < ci-output.txt > safe-logs.txt
```

**Clean database backups**:
```bash
scred --redact "mysql*,postgresql*,mongodb*,redis*" < backup.sql > clean-backup.sql
```

**Safe log aggregation**:
```bash
# Parse and redact, keeping only high-value patterns
find /var/log -name "*.log" -exec scred --redact "CRITICAL,API_KEYS,!test-*" {} \; > aggregated.log
```
