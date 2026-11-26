// SPDX-License-Identifier: MPL-2.0

//! 仅用于调试的 syscall：让用户态程序能在正常启动的内核上驱动 #3896 触发函数，
//! 从而不借助 ktest 就能复现（以后还能记录并重放）这个 bug。这不是 Linux syscall，
//! 号是 KMiri 实验私有的。

use ostd::mm::VmIo;

use super::SyscallReturn;
use crate::prelude::*;

pub(super) fn sys_kmiri_3896(
    buf_addr: Vaddr,
    cap: usize,
    ctx: &Context,
) -> Result<SyscallReturn> {
    // 跑触发函数；若给了缓冲区，就把记录下来的事件时间线拷回用户态，这样完整的插桩顺序
    // 无需任何内核内打印即可观测。返回结果码：bit 0 置位表示 RCU 在过期引用被使用之前就
    // 回收了对象；bits 8.. 是通过那个过期引用读到的值。
    let code = ostd::kmiri_3896::trigger_3896();

    if buf_addr != 0 && cap > 0 {
        let mut events = [0u32; 128];
        let n = ostd::kmiri_3896::copy_timeline(&mut events);
        let k = core::cmp::min(cap, n);
        let user = ctx.user_space();
        for (i, ev) in events.iter().take(k).enumerate() {
            user.write_val(buf_addr + i * core::mem::size_of::<u32>(), ev)?;
        }
    }

    Ok(SyscallReturn::Return(code as _))
}
