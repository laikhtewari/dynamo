#!/bin/bash

# export HF_TOKEN="your-token"

# Once everything is ready, run this:
genai-perf profile     \
        --model meta-llama/Llama-4-Maverick-17B-128E-Instruct     \
        --tokenizer meta-llama/Llama-4-Maverick-17B-128E-Instruct     \
        --endpoint-type chat     \
        --endpoint /v1/chat/completions     \
        --streaming     \
        --url http://trtllm-agg-router-frontend:8000     \
        --synthetic-input-tokens-mean 200000     \
        --synthetic-input-tokens-stddev 0     \
        --output-tokens-mean 100    \
        --output-tokens-stddev 0     \
        --extra-inputs max_tokens:100     \
        --extra-inputs min_tokens:100     \
        --extra-inputs ignore_eos:true     \
        --extra-inputs "{\"nvext\":{\"ignore_eos\":true}}"     \
        --concurrency 1    \
        --warmup-request-count 8     \
        --num-dataset-entries 64     \
        --random-seed 100  \
        --measurement-interval 120000   \
        --artifact-dir /tmp/benchmark-results     \
        --     -v     -H 'Authorization: Bearer NOT USED'     -H 'Accept: text/event-stream'
