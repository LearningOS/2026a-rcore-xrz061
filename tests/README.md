# 补充回归测试

## 第四章

在 ch4 分支，将 `tests/apps/ch4_vm_regression.rs` 复制到
`ci-user/user/src/bin/ch4_vm_regression.rs`，执行官方测试：

```sh
make -C ci-user test CHAPTER=4
```

除了官方检查器全部通过，还需检查输出中包含 `VM regression OK`，且该应用没有 panic。
本机使用 Docker 内的 QEMU 8.2.2，可通过 `PATH=/tmp/rcore-tools:$PATH` 使用已配置的临时包装器。
测例副本只用于本地验证，不修改官方检查器。

## 第五章

在 ch5 分支，将 `tests/apps/ch5_process_regression.rs` 复制到
`ci-user/user/src/bin/ch5_process_regression.rs`，并在本地
`ci-user/user/src/bin/ch5_usertest.rs` 的 `TESTS` 列表中加入
`"ch5_process_regression\0"`。保持 `pid` 数组容量不小于列表长度。
执行官方测试：

```sh
make -C ci-user test CHAPTER=5
```

除官方检查器 15/15、三份报告检查通过和命令成功退出外，还需确认输出
`Process regression OK`，且该应用退出码为 0。该测试覆盖 spawn 非法路径和
跨页路径、最大合法优先级、解除中间页映射后的 fork、父子内存隔离和
退出码回收。检查器会随机替换输出中的 `OK`，因此标记中可能带有数字。
