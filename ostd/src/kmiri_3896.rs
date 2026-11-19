// SPDX-License-Identifier: MPL-2.0

//! 复现已报告的 soundness 洞 #3896 / #1120，在正常启动的 OSTD 内核上由用户态驱动（不是 ktest）。
//!
//! 嵌套的 `DisabledLocalIrqGuard` 乱序 drop，会在内层 guard 还活着时提前打开本地中断——
//! 而内层 guard 本应是“处于原子模式”的证明。`Rcu::read_with` 依赖这个证明，所以窗口一旦打开，
//! 一次真实的任务切换就能让 RCU 宽限期结束、把被引用的内存回收掉，而那个过期引用还在被使用。
//!
//! 宽限期完全靠真实的 `Task::yield_now() -> switch_to_task -> finish_grace_period` 路径推进，
//! 没有人工调用 `finish_grace_period()`。能走到那次切换，正是因为乱序 drop 已经破坏了原子模式；
//! 若按正确的 LIFO 顺序 drop，`might_sleep` 会直接拦下来。
//!
//! 观测不依赖 KMiri：完整事件时间线记录在无锁原子环里、可由 syscall 拷回用户态；另外返回一个
//! 结果码给调用方（调试 syscall），用户态据此判断是否触发。全程不在内核里 print。

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::Relaxed};

use crate::{prelude::*, sync::Rcu, task::Task};

/// 触发时间线上按序记录的事件。用具名枚举：交给用户态的顺序本身就是自解释的，
/// “回收早于使用”这个判据也能不依赖 KMiri 直接检查。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
enum Ev {
    OuterNew = 1,                   // 建外层关中断 guard（was_enabled=true）
    InnerNew = 2,                   // 建内层关中断 guard（was_enabled=false）
    ReadWith = 3,                   // 用内层 guard 读出 RCU 引用 r
    OuterDrop = 4,                  // 乱序 drop 外层 guard，中断被提前打开
    IrqEnabledWhileWitnessLive = 5, // 确认：内层 guard 还活着时中断已开
    MightSleepReturned = 6,         // might_sleep 本应拦截，这里却放行
    Update = 7,                     // 写端替换对象，旧对象进入待回收
    Reclaim = 8,                    // 旧对象开始析构：其 Drop 运行，紧接着 Box 释放底层内存
    Use = 9,                        // 通过已失效的引用 r 读取（UAF 读）
    InnerDrop = 10,                 // drop 内层 guard
}

const N: usize = 128;
static RING: [AtomicU32; N] = [const { AtomicU32::new(0) }; N];
static CUR: AtomicUsize = AtomicUsize::new(0);

/// 由旧对象的 Drop 置位。让触发函数能“等到回收真正发生”（一次真实宽限期完成）再去读，
/// 而不是猜一个固定的让出次数——正是这一点让这个 bug 每次都能触发，而不是碰运气。
static RECLAIMED: AtomicBool = AtomicBool::new(false);

fn reset() {
    CUR.store(0, Relaxed);
    RECLAIMED.store(false, Relaxed);
    for slot in RING.iter() {
        slot.store(0, Relaxed);
    }
}

/// 记录一个事件：只做一次原子自增 + 原子写，无锁、不取锁、不触发切换，因此不会扰动被观测的流程。
fn rec(ev: Ev) {
    let i = CUR.fetch_add(1, Relaxed);
    if i < N {
        RING[i].store(ev as u32, Relaxed);
    }
}

/// 扫描记录下来的时间线，找 UAF 的特征：第一个 `Reclaim` 严格早于过期引用的 `Use`。
/// 这就是不依赖 KMiri 的、“这次启动确实触发了”的证明。
fn reclaim_before_use() -> bool {
    let n = CUR.load(Relaxed).min(N);
    let mut reclaim_at: Option<usize> = None;
    let mut use_at: Option<usize> = None;
    for i in 0..n {
        match RING[i].load(Relaxed) {
            x if x == Ev::Reclaim as u32 => {
                reclaim_at.get_or_insert(i);
            }
            x if x == Ev::Use as u32 => {
                use_at.get_or_insert(i);
            }
            _ => {}
        }
    }
    matches!((reclaim_at, use_at), (Some(r), Some(u)) if r < u)
}

/// 把时间线写到串口。用 `early_println!` 是因为它直接写 `SERIAL_PORT`。注意：在当前
/// run_kernel 配置下它不一定出现在 qemu.log（见 PR 说明），所以主观测走“拷回用户态”那条路，
/// 这个 dump 只是备用。它在临界区结束后（中断已恢复）才调用，这里取串口锁是安全的；
/// 触发窗口内只用无锁的 `rec`。
fn dump(tag: &str, yields: u32) {
    let n = CUR.load(Relaxed).min(N);
    crate::early_println!("[kmiri-3896] timeline {}:", tag);
    for i in 0..n {
        crate::early_println!("    step {}: ev={}", i, RING[i].load(Relaxed));
    }
    crate::early_println!(
        "[kmiri-3896] {}: reclaim_before_use={} yields_until_reclaim={}",
        tag,
        reclaim_before_use(),
        yields,
    );
}

/// RCU 指针后面挂的堆对象。它的 `Drop` 会记录一个 `Reclaim` 事件并置位 `RECLAIMED`；
/// 这个 drop 正是“读者引用还活着时绝不该发生”的那次回收。
struct Payload {
    val: u32,
}

impl Drop for Payload {
    fn drop(&mut self) {
        rec(Ev::Reclaim);
        RECLAIMED.store(true, Relaxed);
    }
}

/// 在真实内核上触发 #3896 的乱序 drop RCU use-after-free，可由用户态经调试 syscall 到达。
///
/// 返回一个小的可观测码，让触发进程不依赖 KMiri 也能看到结果：
///   bit 0      = 回收是否严格早于过期读（UAF 窗口命中）；
///   bits 8..   = 通过过期引用读到的值；
///   bits 16..  = 从 retire 到真正回收期间调用 Task::yield_now 的次数（注意：不是真实切换次数，多数 yield 在本 CPU 无其它可运行任务时不切换）。
pub fn trigger_3896() -> u32 {
    reset();

    let rcu: Rcu<Box<Payload>> = Rcu::new(Box::new(Payload { val: 0xAA }));

    let outer = crate::irq::disable_local();
    rec(Ev::OuterNew);
    let inner = crate::irq::disable_local();
    rec(Ev::InnerNew);

    // 读者引用，其健全性绑定在“inner 仍是有效的原子模式证明”上。
    let r = rcu.read_with(&inner);
    rec(Ev::ReadWith);

    // 出问题的非 LIFO drop：先 drop 外层 guard，中断被重新打开，
    // 尽管内层 guard（还活着）本应继续持有原子模式。
    drop(outer);
    rec(Ev::OuterDrop);

    let enabled = crate::arch::irq::is_local_enabled();
    if enabled {
        rec(Ev::IrqEnabledWhileWitnessLive);
    }

    // 原子模式已被破坏，这里不再拦截潜在的 sleep/切换。
    crate::task::atomic_mode::might_sleep();
    rec(Ev::MightSleepReturned);

    // 写端替换对象，把旧对象交给 RCU 等待回收。
    rcu.update(Box::new(Payload { val: 0xBB }));
    rec(Ev::Update);

    // 用真实任务切换推进宽限期，直到旧对象真的被回收。update 之后一次真实切换就够了：
    // SMP=1 下切出 CPU 的静止状态在 `finish_grace_period` 里完成宽限期，随即执行 drop 回调。
    // 必须循环，是因为 `Task::yield_now` 只有当本 CPU 运行队列里有可运行对端时才会切换
    //（用户态驱动会 fork 一个并等它被调度）；上限用来避免对端缺失时把内核挂死。
    let mut yields = 0u32;
    while !RECLAIMED.load(Relaxed) {
        Task::yield_now();
        yields += 1;
        if yields >= 1_000_000 {
            break;
        }
    }

    // 回收之后再用这个引用：UAF 读。
    let observed = r.val;
    rec(Ev::Use);

    drop(inner);
    rec(Ev::InnerDrop);

    dump("out_of_order", yields);

    (reclaim_before_use() as u32) | ((observed & 0xFF) << 8) | ((yields & 0xFFFF) << 16)
}

/// 把记录下来的事件时间线拷进 `out`（每格一个 `Ev` 码，按发生顺序），返回记录的事件数。
/// 让调试 syscall 能把这份插桩顺序交回用户态，于是时间线可观测、且无需任何内核内打印。
pub fn copy_timeline(out: &mut [u32]) -> usize {
    let n = CUR.load(Relaxed).min(N);
    for (slot, dst) in RING.iter().take(n).zip(out.iter_mut()) {
        *dst = slot.load(Relaxed);
    }
    n
}
