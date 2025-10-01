#!/bin/bash

# docker run -ti --gpus all --network host -v /tmp:/tmp gitlab-master.nvidia.com:5005/dl/ai-dynamo/dynamo-ci:1d30e263cbab2fd00265c7eb043387d98dfc8acf-35883437-trtllm-arm64

# export HF_TOKEN="your-token"


nats-server -js &
etcd --listen-client-urls http://0.0.0.0:2379 --advertise-client-urls http://0.0.0.0:2379 --data-dir /tmp/etcd &

# Frontend
python3 -m dynamo.frontend &

# Worker
python3 -m dynamo.trtllm \
  --model-path meta-llama/Llama-4-Maverick-17B-128E-Instruct \
  --served-model-name meta-llama/Llama-4-Maverick-17B-128E-Instruct \
  --extra-engine-args /tmp/engine_configs/decode.yaml \
  --publish-events-and-metrics
