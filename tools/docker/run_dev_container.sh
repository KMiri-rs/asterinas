#!/bin/bash

# SPDX-License-Identifier: MPL-2.0

set -e

export SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
export ASTER_SRC_DIR=${SCRIPT_DIR}/../..
export CARGO_TOML_PATH=${SCRIPT_DIR}/../../Cargo.toml
export VERSION=$(cat ${ASTER_SRC_DIR}/VERSION)
export IMAGE_NAME="asterinas/asterinas:${VERSION}"

#docker run -it --privileged --network=host -v /dev:/dev -v ${ASTER_SRC_DIR}:/root/asterinas ${IMAGE_NAME}

# 2. 使用 docker compose 启动
docker compose up -d

# 3. “进入”环境
docker compose exec asterinas fish
