Packfile 传输协议
===========================

Git 支持通过 ssh://、git://、http:// 与 file:// 四种传输方式发送 packfile。  
协议分两大类：
- **推送**（客户端 → 服务器）
- **获取**（服务器 → 客户端）

ssh、git、file 三种传输使用**相同**的协议流程；http 协议另见 `http-protocol.txt`。

典型进程映射
-------------
- **获取**：服务器 `upload-pack` ↔ 客户端 `fetch-pack`  
- **推送**：服务器 `receive-pack` ↔ 客户端 `send-pack`

核心目标：服务器先告知自己拥有什么，然后双方协商出**最小**需要传输的数据量。

--------------------------------------------------------------------------------

pkt-line 格式
-------------
下文所有 `PKT-LINE(...)` 均基于 `protocol-common.txt` 定义的 pkt-line 格式。  
发送方**应该**在文本后加 LF，接收方**不得**因缺少 LF 而报错。

--------------------------------------------------------------------------------

传输方式一览
-------------
1. **Git 传输**（git://）  
   简单、无认证，默认启动 `upload-pack`；也可配置为允许 `receive-pack`。  
2. **SSH 传输**（ssh://）  
   通过远程执行 `git-upload-pack` / `git-receive-pack` 完成。  
3. **File 传输**（file://）  
   本地直接启动对应进程，通过管道通信。

--------------------------------------------------------------------------------

Extra Parameters（额外参数）
-----------------------------
客户端可在**首条消息**中附加 `<key>=<value>` 或 `<key>` 形式的参数。  
服务器**必须**忽略不认识的键。  
目前已知唯一有效值：`version=1`。

--------------------------------------------------------------------------------

Git 传输细节
-------------
请求格式（pkt-line）：

```
0033git-upload-pack /project.git\0host=myserver.com\0
```

如需附加 Extra Parameters，再加一个 NUL 后继续：

```
003egit-upload-pack /project.git\0host=myserver.com\0\0version=1\0
```

ABNF 语法：

```
git-proto-request = request-command SP pathname NUL
[ host-parameter NUL ]
[ NUL extra-parameters ]
request-command   = "git-upload-pack" / "git-receive-pack" / "git-upload-archive"
pathname          = *( %x01-ff )          ; 不含 NUL
host-parameter    = "host=" hostname [ ":" port ]
extra-parameters  = 1*extra-parameter
extra-parameter   = 1*( %x01-ff ) NUL
```

host-parameter 用于 git-daemon 的**虚拟主机**功能（`--interpolated-path` 的 `%H/%CH`）。

示例（手动模拟）：

```bash
$ echo -e -n "0039git-upload-pack /schacon/gitbook.git\0host=example.com\0" \
  | nc -v example.com 9418
```

服务器可优雅返回错误：

```
error-line = PKT-LINE("ERR" SP explanation-text)
```

--------------------------------------------------------------------------------

SSH 传输细节
-------------
本质是在远端执行：

```bash
ssh git.example.com "git-upload-pack '/project.git'"
```

URI 路径规则：
- `ssh://host/path` → **绝对**路径 `/path`
- `user@host:path` → 相对用户主目录的 **相对**路径 `path`
- `ssh://host/~alice/path` → 路径 `~alice/path`（**不带**前导 `/`）

Extra Parameters 可通过 `GIT_PROTOCOL` 环境变量传递（ colon-separated ），前提是 `ssh.variant` 配置表明所用 ssh 实现支持。

--------------------------------------------------------------------------------

从服务器获取数据
=================

参考发现（Reference Discovery）
-------------------------------
连接建立后，服务器立即返回：

1. 若 Extra Parameters 含 `version=1`，先回 `000aversion 1`
2. 按 **C locale 排序** 依次列出每个引用及其 obj-id
3. **HEAD 必须**排在首位（若存在）
4. 首条引用后在 NUL 字节处附加**能力列表**
5.  annotated tag 必须立即给出其 peeled 值 `ref^{}`

示例：

```
C: 0044git-upload-pack /schacon/gitbook.git\0host=example.com\0\0version=1\0
S: 000aversion 1
S: 00887217a7c7e582c46cec22a130adf4b9d7d950fba0 HEAD\0multi_ack thin-pack side-band side-band-64k ofs-delta shallow no-progress include-tag
S: 00441d3fcd5ced445d1abc402225c0b8a1299641f497 refs/heads/integration
...
S: 0000
```

ABNF：

```
advertised-refs = *1("version 1")
                  (no-refs / list-of-refs)
                  *shallow
                  flush-pkt
...
```

obj-id 大小写**不敏感**，但双方**必须**使用小写传输。

--------------------------------------------------------------------------------

包文件协商（Packfile Negotiation）
----------------------------------
客户端完成引用发现后可立即发送 `flush-pkt` 结束（如 `ls-remote` 或已最新）。  
否则进入协商阶段，格式：

```
upload-request = want-list
                 *shallow-line
                 *1depth-request
                 [filter-request]
                 flush-pkt
```

规则：
- 所有 `want` 的 obj-id **必须**来自刚才的引用发现
- 至少发送一条 `want`
- `shallow` 行声明客户端**浅克隆**边界
- `deepen` / `deepen-since` / `deepen-not` 控制**深度/时间/排除**截断
- `filter` 用于**部分克隆**（省略某些对象）

随后客户端持续发送 `have` 行：

```
upload-haves = have-list
               compute-end
```

- 最多**32 条**一批，然后 `flush-pkt`
- 收到服务器 `ACK obj-id continue` 可继续
- 若 256 条 `have` 仍无共同祖先，则直接 `done` 放弃协商

服务器 ACK 模式：
- **multi_ack**：找到共同祖先即 `ACK obj-id continue`；准备好后盲 ACK 所有 `have` 再发 `NAK`
- **multi_ack_detailed**：区分 `common` / `ready`
- **无上述能力**：仅对**第一个**共同对象发一次 `ACK obj-id`，之后沉默直到 `done`

客户端发 `done` 后，服务器回复：
- 有共同祖先且启用 multi_ack*：最后 `ACK obj-id`
- 否则：`NAK`
- 出错：`ERR ...`

随后进入**包文件传输**。

--------------------------------------------------------------------------------

包文件数据（Packfile Data）
-------------------------
若协商了 `side-band` 或 `side-band-64k`，数据将**多路复用**：

| 模式 | 每包最大载荷 | 控制码 |
|---|---|---|
| side-band | 999 B + 1 B | 1=数据 2=进度 3=错误 |
| side-band-64k | 65519 B + 1 B | 同上 |

无 side-band 能力则**原始流**直接发送。  
包文件格式详见 `pack-format.txt`。

--------------------------------------------------------------------------------

向服务器推送数据
=================

概述
----
推送时服务器启动 `receive-pack`；客户端先告知**引用更新计划**，再发送**包文件**（含所需对象）。服务器校验、解包、运行钩子，最终**原子**更新引用。

引用发现
--------
与获取阶段**几乎相同**，仅能力列表不同：  
仅可能包含 `report-status`, `delete-refs`, `ofs-delta`, `push-options`。

引用更新请求 & 包文件传输
-------------------------
客户端依次发送：

```
update-requests = *shallow ( command-list | push-cert )
```

命令格式：

```
command = create / delete / update
create  = zero-id SP new-id SP name
delete  = old-id  SP zero-id SP name
update  = old-id  SP new-id  SP name
```

- 若服务器**无** `delete-refs`，客户端**不得**发送 delete 命令
- 若**无** `push-cert`，客户端**不得**发送证书块
- 仅含 delete 时**不发送包文件**
- 含 create/update 时**必须**发包文件（可为**空包**）

推送证书（Push Certificate）
---------------------------
结构：

```
push-cert = PKT-LINE("push-cert" NUL capability-list LF)
            PKT-LINE("certificate version 0.1" LF)
            PKT-LINE("pusher" SP ident LF)
            PKT-LINE("pushee" SP url LF)
            PKT-LINE("nonce" SP nonce LF)
            *PKT-LINE("push-option" SP push-option LF)
            PKT-LINE(LF)
            *PKT-LINE(command LF)
            *PKT-LINE(gpg-signature-lines LF)
            PKT-LINE("push-cert-end" LF)
```

- 证书部分与签名之间**空行**
- 签名是** detached GPG signature**，覆盖证书全部内容
- 用于防重放、防篡改、认证推送者

报告状态（Report Status）
-----------------------
若启用 `report-status`，服务器在**处理完包文件**后返回：

```
report-status = unpack-status
                1*(command-status)
                flush-pkt
```

示例：

```
S: 000eunpack ok\n
S: 0018ok refs/heads/debug\n
S: 002ang refs/heads/master non-fast-forward\n
```

- `unpack ok` / `unpack [error-msg]`
- 每条引用：`ok refname` 或 `ng refname error-msg`

--------------------------------------------------------------------------------

认证说明
--------
协议本身**不含**认证；由底层传输（SSH、HTTPS）在调用 `receive-pack` 前完成。  
若通过 Git 传输（9418 端口）开放 `receive-pack`，则**任何人**皆可写仓库。
