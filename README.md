# SCRED

Best effort secred redaction tool.

## What It Does

Redacts 50+ types of sensitive credentials (API keys, tokens, passwords, etc.) from your text files and logs.

```bash
$ echo "key: sk-proj-1234567890abcdefghijklmnopqrstuvwxyz" | scred
key: sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Input length preserved, only sensitive parts replaced with `x`.
