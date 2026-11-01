# 实验三：进程管理

## 实现概要

迁移时间写入与 mmap/munmap 到进程的 MemorySet。spawn 验证用户路径，直接从 ELF 创建新地址空间，设置父子关系后加入就绪队列。进程初始优先级为 16、stride 为 0；每次选择最小 stride，并增加 BIG_STRIDE/priority，非法优先级返回 -1。采用 u128 步长和共同偏移归一化，避免大优先级步长变零及累计溢出。

## Stride 问答

1. 若使用 8 位整数并直接比较，p2 从 250 增加 10 后回绕为 4；由于 4 < 255，调度器会继续选择 p2，而正确的扩展整数值应为 260，所以下一次本应选择 p1。
2. 所有进程从相同 stride 开始，且 priority >= 2，所以最大单次增量不超过 BIG_STRIDE/2。每次只增加当前最小值：旧的其他值不会超过旧最小值加上该界限，被选进程的新值也不会超过它，因此差值上界得以保持。此论证针对固定的一组就绪进程；新进程加入时还需考虑初始化策略。
3. 若累计值使用整个 u64 环，且真实距离严格小于半环，可以按环上的距离比较。BinaryHeap 默认是最大堆，因此要反转逻辑，让逻辑最小 stride 具有最大堆序：

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let delta = self.0.wrapping_sub(other.0) as i64;
        Some(delta.cmp(&0).reverse())
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
```

例如 8 位情况下，125 与 255 相差 130，超过半环，应视 125 为回绕后的较大值；129 与 255 相差 126，应视 129 为逻辑较小值。上述比较器的前提是半环界限，不能用来比较任意距离的计数值。完整用于 BinaryHeap 时还需实现保持一致的 Eq/Ord，或使用 Reverse 包装自然次序。

本次实现使用线性扫描，FIFO 顺序处理相等 stride，不依赖该比较器。BIG_STRIDE=2^64，用 u128 计算，priority=isize::MAX 时步长仍至少为 2。每次选择后，对所有就绪进程减去共同的最小 stride，再增加选中进程的步长。相对顺序不变，累计值保持有界。spawn 和 fork 新进程均按题目初始化为 stride=0、priority=16，exec 保留现有调度身份。

## 验证

使用 nightly-2024-05-02、QEMU 8.2.2 和仓库提供的 RustSBI 0.4.1，执行 `PATH=/tmp/rcore-tools:$PATH make -C ci-user test CHAPTER=5`。2026-11-01 实测官方检查器输出 15/15，lab1、lab2、lab3 报告检查全部通过，命令退出码为 0。本机 QEMU 由临时包装器调用 Docker 镜像 `rcore-qemu:8.2`，将内核和固件通过 tar 传入容器。

补充回归用例位于 `tests/apps/ch5_process_regression.rs`，加入本地用户测试列表后随上述命令运行，实际输出 `Process regression OK`，退出码为 0。覆盖不存在、空、无效地址和非法 UTF-8 的 spawn 路径，跨页路径，最大合法优先级，munmap 拆分区域后的 fork、父子内存隔离和子进程退出码回收。官方检查器的判定规则未修改。

Stride 测例的实际结果如下，count / priority 的最大值与最小值之比约为 1.037，计数基本按优先级分配：

| priority | count | count / priority |
| --- | ---: | ---: |
| 5 | 14832400 | 2966480 |
| 6 | 17628000 | 2938000 |
| 7 | 20591200 | 2941600 |
| 8 | 23583600 | 2947950 |
| 9 | 26624400 | 2958266 |
| 10 | 30465600 | 3046560 |

最终输出 `ch5 Usertests passed`。框架在初始进程正常退出后打印 `All applications completed!` 并关机，这是本章既有结束路径。

## 荣誉准则与协助说明

1. 本次代码、分析及报告由 OpenAI Codex 按用户要求完成，包含 AI 编程协助，不声明为提交者独立完成。提交者应按课程规定确认是否可提交。
2. 参考课程 README、rCore-Tutorial-Guide 第五章练习与荣誉准则、现有进程/调度框架及官方 Test/Checker，未采用其他学员的实验答案。
3. 以上列出本次实际协助和引用，无法代提交者声明其此前学习、交流或独立工作情况。
4. 所有改动用于正常运行课程实验与验证，不以破坏或妨碍系统为目的；课程对引用、独立完成及妥善保管代码的要求仍适用，本报告如实说明协助来源。
