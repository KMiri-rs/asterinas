#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

# Runs the #3896 trigger many times on one boot and reports how many of them
# actually hit the use-after-free window, to show the trigger is stable.

N=10
pass=0
for i in $(seq 1 $N); do
    echo "[kmiri-3896-user] run $i"
    if /test/kmiri/kmiri_3896; then
        pass=$((pass + 1))
    fi
done
echo "[kmiri-3896-user] triggered $pass / $N"
echo "KMIRI_3896_DONE"
