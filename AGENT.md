
# scred-detector

This crate handles all the logic for pattern detection and redaction
hard rules : 
- SIMD is a first class citizen
- avoid using regex
- prefer using prefixed patterns
- prefer no false positive over completion
- detection on secret values is a first clas citizen.
- it is crucial to keep same length when redacting secrets.
- streaming is the only way to go.
- buffering is acceptable only for look ahead.


# scred-cli

This crate allow user to call the binary using single secrets (detection is done on value)

echo "$OPENAI_API_KEY" | scred
sk-Fxxxxxxxxxxxxxxxxxxxxx

or using a key=value format to protect env

env | scred

LANGSMITH_API_KEY=lsv2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
LANGSMITH_DEPLOYMENT_KEY=lsv2xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
API_KEY=eyJhxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx

using key=value is a specialization of scred-detector in scred-cli


# scred-http

This crate contains all the http logic common to scred-mitm and scred-proxy.
http 1.1 and http2 are first class citizen.
response redaction is possible but not activated by default.

This handle :
- direct connections to an upstream - and handle the protocol versions gracefully (protocol version, tls, etc)
- connecting to an upstrema HTTP_PROXY (or HTTPS_PROXY) and handle the protocol gracefully (protocol version, tls, etc)

This handle the detection and redaction at the protocol level, using streaming.

# scred-proxy

This crate is a http reverse proxy masking secrets from a specific upstream, in headers and in body.
http 1.1 and http2 are first class citizen.


# scred-mitm

This crate is a http proxy, supporting generating SSL certificate on the fly, masking secrets for everytging that goes throught, in headers and in body.
http 1.1 and http2 are first class citizen.