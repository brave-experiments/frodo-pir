## Tools to treat and bucket credentials

Converts sample credential data into the bucketing structure that is used by the Credential Checker service.

Example usage: 

1. Using a YAML config file:
```bash
 $ CONFIG_PATH=./local-config.yaml go run . 2>&1 | tee logs.txt
```

2. Using environment variables:
```bash
 $ NUMBER_BUCKETS=16 CREDS_PATH=./sample-data/creds BUCKETS_PATH=./sample-data/buckets OPRF_KEY=VBiS3Zlp4UjLXLf9nw4GtU0j5LVfA9T+0u31skECPAY= go run .  2>&1 | tee logs.txt
```

