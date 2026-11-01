# 第八章补充验证

检测算法的主机单元测试：

```sh
rustc --edition=2021 --test os/src/sync/deadlock.rs -o /tmp/rcore-deadlock-tests
/tmp/rcore-deadlock-tests
```

将 `tests/apps/ch8_deadlock_regression.rs` 复制到 `ci-user/user/src/bin/`，
在本地 `ch8_usertest.rs` 的 TESTS 数组加入 `"ch8_deadlock_regression\0"`，
然后执行 `make -C ci-user test CHAPTER=8`。确认官方 25/25、五份报告检查通过、
命令退出成功，且补充测试输出 `Deadlock regression OK`、进程退出码为 0。
检查器可能在 OK 后追加随机数字。

补充测试覆盖检测开关、重复拒绝后的恢复、非法开关值和资源编号、零计数信号量、
自旋锁和阻塞锁的双线程依赖环、线程编号复用以及启用检测时的条件变量交接。
只扩展本地用户测试列表，保持官方检查器的判定规则不变。
