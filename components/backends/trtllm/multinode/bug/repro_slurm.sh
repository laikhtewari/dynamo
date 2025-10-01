#!/bin/bash

export IMAGE="gitlab-master.nvidia.com/dl/ai-dynamo/dynamo-ci:1d30e263cbab2fd00265c7eb043387d98dfc8acf-35883437-trtllm-arm64"
export MOUNTS="${PWD}/../../:/mnt,/lustre:/lustre"

#export MODEL_PATH="meta-llama/Llama-4-Maverick-17B-128E-Instruct"
export MODEL_PATH="/lustre/fsw/core_dlfw_ci/kprashanth/meta-llama_Llama-4-Maverick-17B-128E-Instruct/"
export SERVED_MODEL_NAME="meta-llama/Llama-4-Maverick-17B-128E-Instruct"

export ENGINE_CONFIG="/mnt/multinode/bug/agg.yaml"

../srun_aggregated.sh
