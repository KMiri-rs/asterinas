#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

# Drives the asterinas#3781 reproduction: an ordinary user program tampers with the RFLAGS
# saved in its own signal frame and lets the kernel restore it via rt_sigreturn.
/test/kmiri/kmiri_3781
echo "KMIRI_3781_DONE"
