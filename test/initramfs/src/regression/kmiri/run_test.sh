#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

# Observes asterinas#3871 from an ordinary user program: every syscall entry/exit writes the
# BSP `LOCAL_TSS` static, and `/proc/kmiri_3871` reports what happened to its bytes.

/test/kmiri/kmiri_3871
echo "KMIRI_3871_DONE"
