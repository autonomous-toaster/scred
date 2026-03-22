//! Pattern Definitions
//!
//! Single source of truth for ALL secret patterns:
//! - SIMPLE_PREFIX_PATTERNS (26): Pure prefix, no validation
//! - JWT_PATTERNS (1): eyJ prefix + 2 dots structure
//! - PREFIX_VALIDATION_PATTERNS (45): Prefix + charset/length validation
//! - REGEX_PATTERNS (198): Full regex patterns
//!
//! Total: 270 patterns

const std = @import("std");

// ============================================================================
// Struct Definitions
// ============================================================================

pub const SimplePrefixPattern = struct {
    name: []const u8,
    prefix: []const u8,
};

pub const JwtPattern = struct {
    name: []const u8,
    // JWT detection: search for "eyJ" prefix + exactly 2 dots
};

pub const Charset = enum {
    alphanumeric, // a-z, A-Z, 0-9, -, _
    base64, // a-z, A-Z, 0-9, +, /, =
    base64url, // a-z, A-Z, 0-9, -, _, =
    hex, // 0-9, a-f, A-F
    hex_lowercase, // 0-9, a-f
    any, // Non-delimiter characters
};

pub const PrefixValidation = struct {
    name: []const u8,
    prefix: []const u8,
    min_len: usize, // Minimum token length (0 = no limit)
    max_len: usize, // Maximum token length (0 = no limit)
    charset: Charset, // Character set validation
};

pub const RegexPattern = struct {
    name: []const u8,
    pattern: []const u8, // Raw regex pattern string
};

// ============================================================================
// SIMPLE_PREFIX_PATTERNS (26)
// ============================================================================
// Pure prefix patterns: just search for the prefix, no validation needed
// Examples: "sk_live_", "eyJ", "organizations/", etc.

pub const SIMPLE_PREFIX_PATTERNS = [_]SimplePrefixPattern{
    .{ .name = "age-secret-key", .prefix = "AGE-SECRET-KEY-1" },
    .{ .name = "apideck", .prefix = "sk_live_" },
    .{ .name = "artifactoryreferencetoken", .prefix = "cmVmdGtu" },
    .{ .name = "azure-storage", .prefix = "AccountName" },
    .{ .name = "azure-app-config", .prefix = "Endpoint=https://" },
    .{ .name = "coinbase", .prefix = "organizations/" },
    .{ .name = "context7-api-key", .prefix = "ctx7sk_" },
    .{ .name = "context7-secret", .prefix = "ctx7sk-" },
    .{ .name = "fleetbase", .prefix = "flb_live_" },
    .{ .name = "flutterwave-public-key", .prefix = "FLWPUBK_TEST-" },
    .{ .name = "linear-api-key", .prefix = "lin_api_" },
    .{ .name = "linearapi", .prefix = "lin_api_" },
    .{ .name = "openaiadmin", .prefix = "sk-admin-" },
    .{ .name = "pagarme", .prefix = "ak_live_" },
    .{ .name = "planetscale-1", .prefix = "pscale_tkn_" },
    .{ .name = "planetscaledb-1", .prefix = "pscale_pw_" },
    .{ .name = "pypi-upload-token", .prefix = "pypi-AgEIcHlwaS5vcmc" },
    .{ .name = "ramp", .prefix = "ramp_id_" },
    .{ .name = "ramp-1", .prefix = "ramp_sec_" },
    .{ .name = "rubygems", .prefix = "rubygems_" },
    .{ .name = "salad-cloud-api-key", .prefix = "salad_cloud_" },
    .{ .name = "sentry-access-token", .prefix = "bsntrys_" },
    .{ .name = "sentryorgtoken", .prefix = "sntrys_" },
    .{ .name = "stripepaymentintent-2", .prefix = "pk_live_" },
    .{ .name = "travisoauth", .prefix = "travis_" },
    .{ .name = "tumblr-api-key", .prefix = "tumblr_" },
    .{ .name = "upstash-redis", .prefix = "redis_" },
    .{ .name = "vercel-token", .prefix = "vercel_" },
};

// ============================================================================
// JWT_PATTERNS (1)
// ============================================================================
// Generic JWT detector: eyJ prefix + exactly 2 dots structure
// Covers ALL JWT algorithms (HS256, RS256, EdDSA, etc.)

pub const JWT_PATTERNS = [_]JwtPattern{
    .{ .name = "jwt-generic" }, // eyJ + 2 dots = universal JWT
};

// ============================================================================
// PREFIX_VALIDATION_PATTERNS (45)
// ============================================================================
// Prefix + charset/length validation patterns
// Examples: "sk-ant-" + 90-100 chars, "AKCp" + exactly 69 alphanumeric, etc.

pub const PREFIX_VALIDATION_PATTERNS = [_]PrefixValidation{
    .{ .name = "1password-svc-token", .prefix = "ops_eyJ", .min_len = 250, .max_len = 0, .charset = .base64 },
    .{ .name = "anthropic", .prefix = "sk-ant-", .min_len = 90, .max_len = 100, .charset = .any },
    .{ .name = "artifactory-api-key", .prefix = "AKCp", .min_len = 69, .max_len = 69, .charset = .alphanumeric },
    .{ .name = "assertible", .prefix = "assertible_", .min_len = 20, .max_len = 0, .charset = .any },
    .{ .name = "atlassian", .prefix = "AAAAA", .min_len = 20, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "checkr-personal-access-token", .prefix = "chk_live_", .min_len = 40, .max_len = 0, .charset = .base64url },
    .{ .name = "circleci-personal-access-token", .prefix = "ccpat_", .min_len = 40, .max_len = 0, .charset = .base64url },
    .{ .name = "contentful-personal-access-token", .prefix = "CFPAT-", .min_len = 43, .max_len = 43, .charset = .alphanumeric },
    .{ .name = "databricks-token", .prefix = "dapi", .min_len = 32, .max_len = 0, .charset = .hex },
    .{ .name = "digicert-api-key", .prefix = "d7dc", .min_len = 20, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "duffel-api-token", .prefix = "duffel_", .min_len = 43, .max_len = 60, .charset = .any },
    .{ .name = "dynatrace-api-token", .prefix = "dt0c01.", .min_len = 90, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "easypost-api-token", .prefix = "EZAK", .min_len = 54, .max_len = 54, .charset = .alphanumeric },
    .{ .name = "expo-access-token", .prefix = "ExponentPushToken[", .min_len = 60, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "figma-token", .prefix = "figd_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "flutterwave", .prefix = "FLWRSP_TEST_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "gandi-api-key", .prefix = "Ov23li", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "generic-api-key", .prefix = "X-API-KEY:", .min_len = 20, .max_len = 0, .charset = .any },
    .{ .name = "gitee-access-token", .prefix = "glpat-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "github-token", .prefix = "ghp_", .min_len = 36, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "gitlab-token", .prefix = "glpat-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "google-gemini", .prefix = "AIzaSy", .min_len = 33, .max_len = 33, .charset = .alphanumeric },
    .{ .name = "grafana-api-key", .prefix = "eyJrIjoiZWQ", .min_len = 40, .max_len = 0, .charset = .base64 },
    .{ .name = "heroku-api-key", .prefix = "heroku_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "hubspot-api-key", .prefix = "pat-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "huggingface-token", .prefix = "hf_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "mailgun-api-key", .prefix = "key-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "mapbox-token", .prefix = "pk.", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "mailchimp-api-key", .prefix = "us", .min_len = 32, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "minio-access-key", .prefix = "minioadmin", .min_len = 20, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "notion-api-key", .prefix = "secret_", .min_len = 60, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "npm-token", .prefix = "npm_", .min_len = 36, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "okta-api-token", .prefix = "OKTA_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "openai-api-key", .prefix = "sk-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "pagerduty-api-key", .prefix = "U+", .min_len = 20, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "pagertree-api-token", .prefix = "pt_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "planetscale-password", .prefix = "pscale_pw_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "postman-api-key", .prefix = "PMAK-", .min_len = 50, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "sendgrid-api-key", .prefix = "SG.", .min_len = 69, .max_len = 69, .charset = .alphanumeric },
    .{ .name = "shopify-app-password", .prefix = "shpat_", .min_len = 32, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "slack-token", .prefix = "xoxb-", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "snyk-api-token", .prefix = "snyk_", .min_len = 40, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "stripe-api-key", .prefix = "sk_live_", .min_len = 32, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "supabase-api-key", .prefix = "eyJhbGc", .min_len = 40, .max_len = 0, .charset = .base64 },
    .{ .name = "telegram-bot-token", .prefix = "Bot ", .min_len = 30, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "twilio-api-key", .prefix = "AC", .min_len = 34, .max_len = 0, .charset = .alphanumeric },
    .{ .name = "twitch-oauth-token", .prefix = "oauth:", .min_len = 30, .max_len = 0, .charset = .alphanumeric },
};

// ============================================================================
// REGEX_PATTERNS (198)
// ============================================================================
// Full regex patterns from gitleaks
// Used when simple prefix/validation isn't sufficient
// Note: Regex engine TBD (Oniguruma, PCRE, custom, or Zig's regex)

pub const REGEX_PATTERNS = [_]RegexPattern{
    .{ .name = "1password-service-account-token", .pattern = "ops_eyJ[a-zA-Z0-9+/]{250,}={0,3}" },
    .{ .name = "adafruitio", .pattern = "\\b(aio\\_[a-zA-Z0-9]{28})\\b" }, // could be prefix with validation or just prefix + 28
    .{ .name = "age-secret-key", .pattern = "AGE-SECRET-KEY-1[QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L]{58}" }, // could be prefix with validation or just prefix + 58
    .{ .name = "aha", .pattern = "\\b([A-Za-z0-9](?:[A-Za-z0-9\\-]{0,61}[A-Za-z0-9])\\.aha\\.io)" },
    .{ .name = "airtable-api-key", .pattern = "\\\\b(pat[[:alnum:]]{14}\\\\.[a-f0-9]{64})\\\\b" },
    .{ .name = "airtableoauth", .pattern = "\\b([[:alnum:]]+\\.v1\\.[a-zA-Z0-9_-]+\\.[a-f0-9]+)\\b" },
    .{ .name = "alibaba", .pattern = "\\b([a-zA-Z0-9]{30})\\b" },
    .{ .name = "anthropic", .pattern = "\\b(sk-ant-(?:admin01|api03)-[\\w\\-]{93}AA)\\b" }, // could be multiple prefixes (sk-ant, sk-ant-admin, etc) with validation or just prefix + 95
    .{ .name = "anypoint", .pattern = "\\b([0-9a-z]{8}-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{4}-[0-9a-z]{12})\\b" },
    .{ .name = "api_key_header", .pattern = "(?i)(?:X-API-KEY|X-API-KEY-HEADER):\\s*([A-Za-z0-9\\-._~+\\/]+=*)" },
    .{ .name = "apideck", .pattern = "\\b(sk_live_[a-z0-9A-Z-]{93})\\b" }, // could be prefix with validation or just prefix + 93
    .{ .name = "apify", .pattern = "\\b(apify\\_api\\_[a-zA-Z-0-9]{36})\\b" }, // could be multiple prefixes with validation or just prefix + 36
    .{ .name = "artifactory-api-key", .pattern = "\\\\bAKCp[A-Za-z0-9]{69}\\\\b" },
    .{ .name = "artifactoryreferencetoken", .pattern = "\\b(cmVmdGtu[A-Za-z0-9]{56})\\b" },
    .{ .name = "artifactoryreferencetoken-1", .pattern = "\\b([A-Za-z0-9][A-Za-z0-9\\-]{0,61}[A-Za-z0-9]\\.jfrog\\.io)" },
    .{ .name = "auth0managementapitoken", .pattern = "\\b(ey[a-zA-Z0-9._-]+)\\b" },
    .{ .name = "auth0managementapitoken-1", .pattern = "([a-zA-Z0-9\\-]{2,16}\\.[a-zA-Z0-9_-]{2,3}\\.auth0\\.com)" },
    .{ .name = "auth0oauth", .pattern = "\\b([a-zA-Z0-9_-]{64,})\\b" },
    .{ .name = "auth0oauth-1", .pattern = "\\b([a-zA-Z0-9][a-zA-Z0-9._-]*auth0\\.com)\\b" },
    .{ .name = "authorization_header", .pattern = "(?i)Authorization:\\s*(?:Bearer|Basic|Token)\\s+([A-Za-z0-9\\-._~+\\/]+=*)" },
    .{ .name = "aws-access-token", .pattern = "((?:A3T[A-Z0-9]|AKIA|ASIA|ABIA|ACCA)[A-Z0-9]{16})" },
    .{ .name = "aws-session-token", .pattern = "([A-Za-z0-9/+=]{356,})" },
    .{ .name = "azure-ad-client-secret", .pattern = "(?:^|[\\\\\\\\\\" },
    .{ .name = "azure_batch-1", .pattern = "[A-Za-z0-9+/=]{88}" },
    .{ .name = "azure_cosmosdb", .pattern = "([A-Za-z0-9]{86}==)" },
    .{ .name = "azure_cosmosdb-1", .pattern = "([a-z0-9-]{3,44}\\.(?:documents|table\\.cosmos)\\.azure\\.com)" },
    .{ .name = "azure_entra", .pattern = "([\\w-]+\\.onmicrosoft\\.com)" },
    .{ .name = "azure_openai", .pattern = "(?i)([a-z0-9-]+\\.openai\\.azure\\.com)" },
    .{ .name = "azure_storage", .pattern = "AccountName=(?P<account_name>[^;]+);AccountKey" },
    .{ .name = "azureappconfigconnectionstring", .pattern = "Endpoint=(https:\\/\\/[a-zA-Z0-9-]+\\.azconfig\\.io);Id=([a-zA-Z0-9+\\/=]+);Secret=([a-zA-Z0-9+\\/=]+)" },
    .{ .name = "azurecontainerregistry", .pattern = "([a-z0-9][a-z0-9-]{1,100}[a-z0-9])\\.azurecr\\.io" },
    .{ .name = "azurecontainerregistry-1", .pattern = "\\b[a-zA-Z0-9+/]{42}\\+ACR[a-zA-Z0-9]{6}\\b" },
    .{ .name = "bitbucketapppassword", .pattern = "(?:^|[^A-Za-z0-9-_])(?P<username>[A-Za-z0-9-_]{1,30}):(?P<password>ATBB[A-Za-z0-9_=.-]+)\\b" },
    .{ .name = "bitbucketapppassword-1", .pattern = "https://(?P<username>[A-Za-z0-9-_]{1,30}):(?P<password>ATBB[A-Za-z0-9_=.-]+)@bitbucket\\.org" },
    .{ .name = "caflou", .pattern = "\\b(eyJhbGciOiJIUzI1NiJ9[a-zA-Z0-9._-]{135})\\b" },
    .{ .name = "clickhelp", .pattern = "\\b([0-9A-Za-z-]{3,20}\\.(?:try\\.)?clickhelp\\.co)\\b" },
    .{ .name = "clickhouse-cloud-api-secret-key", .pattern = "\\\\b(4b1d[A-Za-z0-9]{38})\\\\b" },
    .{ .name = "clojars-api-token", .pattern = "(?i)CLOJARS_[a-z0-9]{60}" }, // could be prefix with validation or just prefix + 60
    .{ .name = "closecrm", .pattern = "\\b(api_[a-z0-9A-Z.]{45})\\b" },
    .{ .name = "cloudflarecakey", .pattern = "\\b(v1\\.0-[A-Za-z0-9-]{171})\\b" },
    .{ .name = "coinbase", .pattern = "\\b(organizations\\\\*/\\w{8}-\\w{4}-\\w{4}-\\w{4}-\\w{12}\\\\*/apiKeys\\\\*/\\w{8}-\\w{4}-\\w{4}-\\w{4}-\\w{12})\\b" },
    .{ .name = "contentfulpersonalaccesstoken", .pattern = "\\b(CFPAT-[a-zA-Z0-9_\\-]{43})\\b" }, // could be prefix with validation or just prefix + 43
    .{ .name = "couchbase", .pattern = "\\b(cb\\.[a-z0-9]+\\.cloud\\.couchbase\\.com)\\b" },
    .{ .name = "databrickstoken", .pattern = "\\b([a-z0-9-]+(?:\\.[a-z0-9-]+)*\\.(cloud\\.databricks\\.com|gcp\\.databricks\\.com|azuredatabricks\\.net))\\b" },
    .{ .name = "databrickstoken-1", .pattern = "\\b(dapi[0-9a-f]{32}(-\\d)?)\\b" }, // could be prefix with validation or just prefix + 32 + optional -digit
    .{ .name = "datadogapikey", .pattern = "\\b(api(?:\\.[a-z0-9-]+)?\\.(?:datadoghq|ddog-gov)\\.(com|eu))\\b" },
    .{ .name = "datadogtoken", .pattern = "\\b(api(?:\\.[a-z0-9-]+)?\\.(?:datadoghq|ddog-gov)\\.(com|eu))\\b" },
    .{ .name = "deno", .pattern = "\\b(dd[pw]_[a-zA-Z0-9]{36})\\b" }, // could be multiple prefixes with validation or just prefix + 36
    .{ .name = "deputy", .pattern = "\\b([0-9a-z]{1,}\\.as\\.deputy\\.com)\\b" },
    .{ .name = "dfuse", .pattern = "\\b(web\\_[0-9a-z]{32})\\b" }, // could be prefix with validation or just prefix + 32
    // ... (50/198 patterns)
    .{ .name = "digitaloceanv2", .pattern = "\\b((?:dop|doo|dor)_v1_[a-f0-9]{64})\\b" }, // could be multiple prefixes with validation or just prefix + 64
    .{ .name = "documo", .pattern = "\\b(ey[a-zA-Z0-9]{34}.ey[a-zA-Z0-9]{154}.[a-zA-Z0-9_-]{43})\\b" },
    .{ .name = "doppler-api-token", .pattern = "dp\\\\.pt\\\\.(?i)[a-z0-9]{43}" },
    .{ .name = "duffel-api-token", .pattern = "duffel_(?:test|live)_(?i)[a-z0-9_\\\\-=]{43}" },
    .{ .name = "dynatrace-api-token", .pattern = "dt0c01\\\\.(?i)[a-z0-9]{24}\\\\.[a-z0-9]{64}" },
    .{ .name = "easypost-api-token", .pattern = "\\\\bEZAK(?i)[a-z0-9]{54}\\\\b" },
    .{ .name = "endorlabs", .pattern = "\\b(endr\\+[a-zA-Z0-9-]{16})\\b" },
    .{ .name = "fleetbase", .pattern = "\\b(flb_live_[0-9a-zA-Z]{20})\\b" },
    .{ .name = "flexport", .pattern = "\\b(shltm_[0-9a-zA-Z-_]{40})" },
    .{ .name = "flightlabs", .pattern = "\\b(eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9\\.ey[a-zA-Z0-9_-]+\\.[a-zA-Z0-9_-]{86})\\b" },
    .{ .name = "flutterwave-public-key", .pattern = "FLWPUBK_TEST-(?i)[a-h0-9]{32}-X" },
    .{ .name = "flyio", .pattern = "\\b(FlyV1 fm\\d+_[A-Za-z0-9+\\/=,_-]{500,700})\\b" },
    .{ .name = "frameio-api-token", .pattern = "fio-u-(?i)[a-z0-9\\\\-_=]{64}" },
    .{ .name = "freshdesk", .pattern = "\\b([0-9a-z-]{1,}\\.freshdesk\\.com)\\b" },
    .{ .name = "ftp", .pattern = "\\bftp://[\\S]{3,50}:([\\S]{3,50})@[-.%\\w\\/:]+\\b" },
    .{ .name = "gcpapplicationdefaultcredentials", .pattern = "\\{[^{]+client_secret[^}]+\\}" },
    .{ .name = "gemini", .pattern = "\\b((?:master-|account-)[0-9A-Za-z]{20})\\b" },
    .{ .name = "github-pat", .pattern = "ghp_[0-9a-zA-Z]{36,}" }, // could be prefix with validation or just prefix + 36
    .{ .name = "github-oauth", .pattern = "gho_[0-9a-zA-Z]{36,}" }, // could be prefix with validation or just prefix + 36
    .{ .name = "github-user", .pattern = "ghu_[0-9a-zA-Z]{36,}" }, // could be prefix with validation or just prefix + 36
    .{ .name = "github-server", .pattern = "ghs_[0-9a-zA-Z]{36,}" },
    .{ .name = "github-refresh", .pattern = "ghr_[0-9a-zA-Z]{36,}" }, // could be prefix with validation or just prefix + 36
    .{ .name = "gitlab-cicd-job-token", .pattern = "glcbt-[0-9a-zA-Z]{1,5}_[0-9a-zA-Z_-]{20}" }, // could be prefix with validation
    .{ .name = "googlegemini", .pattern = "\\b(AIzaSy[A-Za-z0-9_-]{33})" },
    .{ .name = "googleoauth2", .pattern = "\\b(ya29\\.(?i:[a-z0-9_-]{10,}))(?:[^a-z0-9_-]|\\z)" },
    .{ .name = "grafana", .pattern = "\\b(glc_eyJ[A-Za-z0-9+\\/=]{60,160})" },
    .{ .name = "grafanaserviceaccount", .pattern = "\\b(glsa_[0-9a-zA-Z_]{41})\\b" },
    .{ .name = "grafanaserviceaccount-1", .pattern = "\\b([a-zA-Z0-9-]+\\.grafana\\.net)\\b" },
    .{ .name = "graphcms", .pattern = "\\b(ey[a-zA-Z0-9]{73}.ey[a-zA-Z0-9]{365}.[a-zA-Z0-9_-]{683})\\b" },
    .{ .name = "groq", .pattern = "\\b(gsk_[a-zA-Z0-9]{52})\\b" },
    .{ .name = "harness-api-key", .pattern = "(?:pat|sat)\\\\.[a-zA-Z0-9_-]{22}\\\\.[a-zA-Z0-9]{24}\\\\.[a-zA-Z0-9]{20}" },
    .{ .name = "hashicorp-tf-api-token", .pattern = "(?i)[a-z0-9]{14}\\\\.(?-i:atlasv1)\\\\.[a-z0-9\\\\-_=]{60,70}" },
    .{ .name = "hasura", .pattern = "\\b([a-zA-Z0-9-]+\\.hasura\\.app)\\b" },
    .{ .name = "huggingface", .pattern = "\\b(?:hf_|api_org_)[a-zA-Z0-9]{34}\\b" },
    .{ .name = "intra42", .pattern = "\\b(s-s4t2(?:ud|af)-[a-f0-9]{64})\\b" },
    .{ .name = "intra42-1", .pattern = "\\b(u-s4t2(?:ud|af)-[a-f0-9]{64})\\b" },
    .{ .name = "invoiceocean", .pattern = "\\b([0-9a-z]{1,}\\.invoiceocean\\.com)\\b" },
    .{ .name = "jwt", .pattern = "((?:eyJ|ewogIC|ewoid)[A-Za-z0-9_-]{12,}={0,2}\\.(?:eyJ|ewo)[A-Za-z0-9_-]{12,}={0,2}\\.[A-Za-z0-9+/\\-_\\.]+={0,2})" },
    .{ .name = "kanban", .pattern = "\\b([0-9a-z]{1,}\\.kanbantool\\.com)\\b" },
    .{ .name = "klaviyo", .pattern = "\\b(pk_[[:alnum:]]{34})\\b" },
    .{ .name = "langsmith", .pattern = "\\b(lsv2_(?:pt|sk)_[a-f0-9]{32}_[a-f0-9]{10})\\b" },
    .{ .name = "launchdarkly", .pattern = "\\b((?:api|sdk)-[a-z0-9]{8}-[a-z0-9]{4}-4[a-z0-9]{3}-[a-z0-9]{4}-[a-z0-9]{12})\\b" },
    .{ .name = "ldap", .pattern = "\\b(?i)ldaps?://[\\S]+\\b" },
    .{ .name = "linear-api-key", .pattern = "lin_api_(?i)[a-z0-9]{40}" },
    .{ .name = "linearapi", .pattern = "\\b(lin_api_[0-9A-Za-z]{40})\\b" },
    .{ .name = "locationiq", .pattern = "\\b(pk\\.[a-zA-Z-0-9]{32})\\b" },
    .{ .name = "loggly", .pattern = "\\b([a-zA-Z0-9-]+\\.loggly\\.com)\\b" },
    .{ .name = "mailchimp", .pattern = "[0-9a-f]{32}-us[0-9]{1,2}" },
    .{ .name = "mailgun", .pattern = "\\b(key-[a-z0-9]{32})\\b" },
    .{ .name = "mailgun-1", .pattern = "\\b([a-f0-9]{32}-[a-f0-9]{8}-[a-f0-9]{8})\\b" },
    // ... (100/198 patterns)
    .{ .name = "mapbox", .pattern = "([ps]k\\.[a-zA-Z0-9]{20,})" },
    .{ .name = "mapbox-1", .pattern = "\\b(sk\\.[a-zA-Z-0-9\\.]{80,240})\\b" },
    .{ .name = "mite", .pattern = "\\b([0-9a-z-]{1,}.mite.yo.lk)\\b" },
    .{ .name = "mongodb", .pattern = "\\b(mongodb(?:\\+srv)?://(?P<username>\\S{3,50}):(?P<password>\\S{3,88})@(?P<host>[-.%\\w]+(?::\\d{1,5})?(?:,[-.%\\w]+(?::\\d{1,5})?)*)(?:/(?P<authdb>[\\w-]+)?(?P<options>\\?\\w+=[\\w@/.$-]+(?:&(?:amp;)?\\w+=[\\w@/.$-]+)*)?)?)(?:\\b|$)" },
    .{ .name = "ngc-1", .pattern = "\\b([[:alnum:]]{26}:[[:alnum:]]{8}-[[:alnum:]]{4}-[[:alnum:]]{4}-[[:alnum:]]{4}-[[:alnum:]]{12})\\b" },
    .{ .name = "nightfall", .pattern = "\\b(NF\\-[a-zA-Z0-9]{32})\\b" },
    .{ .name = "notion", .pattern = "\\b(secret_[A-Za-z0-9]{43})\\b" },
    .{ .name = "npmtokenv2", .pattern = "(npm_[0-9a-zA-Z]{36})" },
    .{ .name = "nvapi", .pattern = "\\b(nvapi-[a-zA-Z0-9_-]{64})\\b" },
    .{ .name = "okta", .pattern = "\\b[a-z0-9-]{1,40}\\.okta(?:preview|-emea){0,1}\\.com\\b" },
    .{ .name = "okta-1", .pattern = "\\b00[a-zA-Z0-9_-]{40}\\b" },
    .{ .name = "openai", .pattern = "(sk-(?:proj-|svcacct-|admin-)?[a-zA-Z0-9_-]{20,})" },
    .{ .name = "openaiadmin", .pattern = "(sk-admin-[a-zA-Z0-9_-]{58,})" },
    .{ .name = "litellm", .pattern = "\\b(sk-lk-[a-zA-Z0-9_-]{20,})\\b" },
    .{ .name = "openshift-user-token", .pattern = "\\\\b(sha256~[\\\\w-]{43})(?:[^\\\\w-]|\\\\z)" },
    .{ .name = "openvpn", .pattern = "\\b([a-zA-Z0-9_-]{64,})\\b" },
    .{ .name = "pagarme", .pattern = "\\b(ak_live_[a-zA-Z0-9]{30})\\b" },
    .{ .name = "paypaloauth", .pattern = "\\b([A-Za-z0-9_\\.]{7}-[A-Za-z0-9_\\.]{72}|[A-Za-z0-9_\\.]{5}-[A-Za-z0-9_\\.]{38})\\b" },
    .{ .name = "paypaloauth-1", .pattern = "\\b([A-Za-z0-9_\\.\\-]{44,80})\\b" },
    .{ .name = "paystack", .pattern = "\\b(sk\\_[a-z]{1,}\\_[A-Za-z0-9]{40})\\b" },
    .{ .name = "perplexity-api-key", .pattern = "\\\\b(pplx-[a-zA-Z0-9]{48})(?:[\\\\x60\\" },
    .{ .name = "planetscale", .pattern = "\\bpscale_[A-Za-z0-9_]{16,}\\b" },
    .{ .name = "planetscale-1", .pattern = "\\bpscale_tkn_[A-Za-z0-9_]{43}\\b" },
    .{ .name = "planetscaledb", .pattern = "\\b[a-z0-9]{20}\\b" },
    .{ .name = "planetscaledb-1", .pattern = "\\bpscale_pw_[A-Za-z0-9_]{43}\\b" },
    .{ .name = "planetscaledb-2", .pattern = "\\b(aws|gcp)\\.connect\\.psdb\\.cloud\\b" },
    .{ .name = "postgres", .pattern = "\\b(?i)(postgres(?:ql)?)://\\S+\\b" },
    .{ .name = "posthog", .pattern = "\\b(phx_[a-zA-Z0-9_]{43})\\b" },
    .{ .name = "postman", .pattern = "\\b(PMAK-[a-zA-Z-0-9]{59})\\b" },
    .{ .name = "prefect", .pattern = "\\b(pnu_[a-zA-Z0-9]{36})\\b" },
    .{ .name = "private-key", .pattern = "(?i)-----BEGIN[ A-Z0-9_-]{0,100}PRIVATE KEY(?: BLOCK)?-----[\\\\s\\\\S-]{64,}?KEY(?: BLOCK)?-----" },
    .{ .name = "privatekey", .pattern = "(?i)-----\\s*?BEGIN[ A-Z0-9_-]*?PRIVATE KEY\\s*?-----[\\s\\S]*?----\\s*?END[ A-Z0-9_-]*? PRIVATE KEY\\s*?-----" },
    .{ .name = "pubnubpublishkey", .pattern = "\\b(pub-c-[0-9a-z]{8}-[0-9a-z]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12})\\b" },
    .{ .name = "pubnubpublishkey-1", .pattern = "\\b(sub-c-[0-9a-z]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12})\\b" },
    .{ .name = "pubnubsubscriptionkey", .pattern = "\\b(sub-c-[0-9a-z]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12})\\b" },
    .{ .name = "pulumi", .pattern = "\\b(pul-[a-z0-9]{40})\\b" },
    .{ .name = "pypi-upload-token", .pattern = "pypi-AgEIcHlwaS5vcmc[\\\\w-]{50,1000}" },
    .{ .name = "ramp", .pattern = "\\b(ramp_id_[[:alnum:]]{40})\\b" },
    .{ .name = "ramp-1", .pattern = "\\b(ramp_sec_[[:alnum:]]{48})\\b" },
    .{ .name = "razorpay", .pattern = "(?i)\\brzp_live_[A-Za-z0-9]{14}\\b" },
    .{ .name = "razorpay-1", .pattern = "\\brzp_(?:live|test)_[A-Za-z0-9]{15,}\\b" },
    .{ .name = "readme", .pattern = "(rdme_[a-z0-9]{70})" },
    .{ .name = "reallysimplesystems", .pattern = "\\b(ey[a-zA-Z0-9-._]{153}.ey[a-zA-Z0-9-._]{916,1000})\\b" },
    .{ .name = "rechargepayments", .pattern = "\\bsk(_test)?_(1|2|3|5|10)x[123]_[0-9a-fA-F]{64}\\b" },
    .{ .name = "rechargepayments-1", .pattern = "\\b[0-9a-fA-F]{56}\\b" },
    .{ .name = "rechargepayments-2", .pattern = "\\b[0-9a-fA-F]{32}\\b" },
    .{ .name = "redis", .pattern = "\\bredi[s]{1,2}://[\\S]{3,50}:([\\S]{3,50})@[-.%\\w\\/:]+\\b" },
    .{ .name = "replicate", .pattern = "\\b(r8_[0-9A-Za-z-_]{37})\\b" },
    .{ .name = "robinhoodcrypto", .pattern = "\\b(rh-api-[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})\\b" },
    .{ .name = "robinhoodcrypto-1", .pattern = "\\b(?:[A-Za-z0-9+\\/]{8,})(?:[A-Za-z0-9+\\/]{2}==|[A-Za-z0-9+\\/]{3}=)\\b" },
    // ... (150/198 patterns)
    .{ .name = "rootly", .pattern = "\\b(rootly_[a-f0-9]{64})\\b" },
    .{ .name = "rubygems", .pattern = "\\b(rubygems_[a-zA0-9]{48})\\b" },
    .{ .name = "saladcloudapikey", .pattern = "\\b(salad_cloud_[0-9A-Za-z]{1,7}_[0-9A-Za-z]{7,235})\\b" },
    .{ .name = "salesforce", .pattern = "\\b00[a-zA-Z0-9]{13}![a-zA-Z0-9_.]{96}\\b" },
    .{ .name = "salesforceoauth2-1", .pattern = "\\b(3MVG9[0-9a-zA-Z._+/=]{80,251})" },
    .{ .name = "salesforcerefreshtoken", .pattern = "(?i)\\b(5AEP861[a-zA-Z0-9._=]{80,})\\b" },
    .{ .name = "salesforcerefreshtoken-1", .pattern = "\\b(3MVG9[0-9a-zA-Z._+/=]{80,251})" },
    .{ .name = "saucelabs", .pattern = "\\b(api\\.(?:us|eu)-(?:west|east|central)-[0-9].saucelabs\\.com)\\b" },
    .{ .name = "sendgrid", .pattern = "\\bSG\\.[\\w\\-]{20,24}\\.[\\w\\-]{39,50}\\b" },
    .{ .name = "sendinbluev2", .pattern = "\\b(xkeysib\\-[A-Za-z0-9_-]{81})\\b" },
    .{ .name = "sentry-access-token", .pattern = "\\\\bsntrys_eyJpYXQiO[a-zA-Z0-9+/]{10,200}(?:LCJyZWdpb25fdXJs|InJlZ2lvbl91cmwi|cmVnaW9uX3VybCI6)[a-zA-Z0-9+/]{10,200}={0,2}_[a-zA-Z0-9+/]{43}(?:[^a-zA-Z0-9+/]|\\\\z)" },
    .{ .name = "sentryorgtoken", .pattern = "\\b(sntrys_eyJ[a-zA-Z0-9=_+/]{197})\\b" },
    .{ .name = "shopify-shared-secret", .pattern = "shpss_[a-fA-F0-9]{32}" },
    .{ .name = "sidekiq-secret", .pattern = "(?i)\\\\bhttps?://([a-f0-9]{8}:[a-f0-9]{8})@(?:gems.contribsys.com|enterprise.contribsys.com)(?:[\\\\/|\\\\#|\\\\?|:]|$)" },
    .{ .name = "signable", .pattern = "(?i)([a-z]{2})signable" },
    .{ .name = "signalwire", .pattern = "\\b([0-9a-z-]{3,64}\\.signalwire\\.com)\\b" },
    .{ .name = "slack-bot-token", .pattern = "xoxb-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*" },
    .{ .name = "sourcegraph", .pattern = "\\b(sgp_(?:[a-fA-F0-9]{16}|local)_[a-fA-F0-9]{40}|sgp_[a-fA-F0-9]{40}|[a-fA-F0-9]{40})\\b" },
    .{ .name = "sourcegraphcody", .pattern = "\\b(slk_[a-f0-9]{64})\\b" },
    .{ .name = "squareapp", .pattern = "(?:sandbox-)?sq0i[a-z]{2}-[0-9A-Za-z_-]{22,43}" },
    .{ .name = "squareapp-1", .pattern = "(?:sandbox-)?sq0c[a-z]{2}-[0-9A-Za-z_-]{40,50}" },
    .{ .name = "squareup", .pattern = "\\b(sq0idp-[0-9A-Za-z]{22})\\b" },
    .{ .name = "stripe", .pattern = "[rs]k_live_[a-zA-Z0-9]{20,247}" },
    .{ .name = "stripepaymentintent", .pattern = "\\b(pi_[a-zA-Z0-9]{24}_secret_[a-zA-Z0-9]{25})\\b" },
    .{ .name = "stripepaymentintent-1", .pattern = "\\b([rs]k_live_[a-zA-Z0-9]{20,247})\\b" },
    .{ .name = "stripepaymentintent-2", .pattern = "\\b(pk_live_[a-zA-Z0-9]{20,247})\\b" },
    .{ .name = "sumologickey", .pattern = "(?i)api\\.(?:au|ca|de|eu|fed|jp|kr|in|us2)\\.sumologic\\.com" },
    .{ .name = "supabasetoken", .pattern = "\\b(sbp_[a-z0-9]{40})\\b" },
    .{ .name = "tableau", .pattern = "\\b([A-Za-z0-9+/]{22}==:[A-Za-z0-9]{32})\\b" },
    .{ .name = "tableau-1", .pattern = "\\b([a-zA-Z0-9\\-]+\\.online\\.tableau\\.com)\\b" },
    .{ .name = "tailscale", .pattern = "\\btskey-[a-z]+-[0-9A-Za-z_]+-[0-9A-Za-z_]+\\b" },
    .{ .name = "terraformcloudpersonaltoken", .pattern = "\\b([A-Za-z0-9]{14}.atlasv1.[A-Za-z0-9]{67})\\b" },
    .{ .name = "trufflehogenterprise", .pattern = "\\bthog-key-[0-9a-f]{16}\\b" },
    .{ .name = "trufflehogenterprise-1", .pattern = "\\bthog-secret-[0-9a-f]{32}\\b" },
    .{ .name = "trufflehogenterprise-2", .pattern = "\\b[a-z]+-[a-z]+-[a-z]+\\.[a-z][0-9]\\.[a-z]+\\.trufflehog\\.org\\b" },
    .{ .name = "twilio-api-key", .pattern = "SK[0-9a-fA-F]{32}" },
    .{ .name = "twilioapikey", .pattern = "\\bSK[a-zA-Z0-9]{32}\\b" },
    .{ .name = "twilioapikey-1", .pattern = "\\b[0-9a-zA-Z]{32}\\b" },
    .{ .name = "ubidots", .pattern = "\\b(BBFF-[0-9a-zA-Z]{30})\\b" }, // could be prefix with validation or just prefix + 30
    .{ .name = "uri", .pattern = "\\bhttps?:\\/\\/[\\w!#$%&()*+,\\-./;<=>?@[\\\\\\]^_{|}~]{0,50}:([\\w!#$%&()*+,\\-./:;<=>?[\\\\\\]^_{|}~]{3,50})@[a-zA-Z0-9.-]+(?:\\.[a-zA-Z]{2,})?(?::\\d{1,5})?[\\w/]+\\b" },
    .{ .name = "voiceflow", .pattern = "\\b(VF\\.(?:(?:DM|WS)\\.)?[a-fA-F0-9]{24}\\.[a-zA-Z0-9]{16})\\b" },
    .{ .name = "webexbot", .pattern = "([a-zA-Z0-9]{64}_[a-zA-Z0-9]{4}_[a-zA-Z0-9]{8}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{4}-[a-zA-Z0-9]{12})" },
    .{ .name = "xai", .pattern = "\\b(xai-[0-9a-zA-Z_]{80})\\b" }, // could be prefix with validation or just prefix + 80
    .{ .name = "zendeskapi-1", .pattern = "\\b([a-zA-Z-0-9]{3,25}\\.zendesk\\.com)\\b" },
    .{ .name = "zohocrm", .pattern = "\\b(1000\\.[a-f0-9]{32}\\.[a-f0-9]{32})\\b" },
    .{ .name = "zulipchat-1", .pattern = "(?i)\\b([a-z0-9-]+\\.zulip(?:chat)?\\.com|chat\\.zulip\\.org)\\b" },
    .{ .name = "reserved-future-pattern-slot", .pattern = "(?!)" },
    .{ .name = "uri-credentials", .pattern = "(?:[a-z]+)://(?:[^:]+):([^\\s@]+)@" },
};

// ============================================================================
// Pattern Counts
// ============================================================================

pub const SIMPLE_PREFIX_COUNT = SIMPLE_PREFIX_PATTERNS.len; // 26
pub const JWT_COUNT = JWT_PATTERNS.len; // 1
pub const PREFIX_VALIDATION_COUNT = PREFIX_VALIDATION_PATTERNS.len; // 45
pub const REGEX_COUNT = REGEX_PATTERNS.len; // 198
pub const TOTAL_PATTERNS = SIMPLE_PREFIX_COUNT + JWT_COUNT + PREFIX_VALIDATION_COUNT + REGEX_COUNT; // 270
