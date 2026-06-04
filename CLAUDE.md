# ratatui-sectioned-list

## 版本发布流程

发布新版本时，按以下步骤执行（用户说"发布 / release / 更新 crates"即走此流程）：

1. **改版本号** — 编辑 `Cargo.toml` 的 `version`（semver）。当前发布版的源码 = GitHub tag = crates.io，三者要一致。
2. **提交并 push** 所有改动到 `main`（crates.io 不接受未提交改动）。
3. **本地校验** — `cargo test`、`cargo clippy --all-targets -- -D warnings`、`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` 都要通过（CI 也跑这三项）。
4. **发布到 crates.io**：
   - token 存在 repo 根目录的 `.env`（`CARGO_REGISTRY_TOKEN=...`），已被 `.gitignore` 忽略，**不要提交**。
   - 发布命令：`set -a; . ./.env; set +a; cargo publish`
   - 发布前可先 `cargo publish --dry-run` 检查打包内容（`exclude` 已排除 `.github/`、`SCR-*.png`、`.gitignore`）。
   - 版本不可变：发布后只能 `cargo yank` 撤回，不能覆盖。
5. **打 tag**：`git tag -f vX.Y.Z HEAD && git push -f origin vX.Y.Z`，确保 tag 指向实际发布的 commit（含所有修复），而不是更早的 "Release" commit。
6. **创建 GitHub release**：`gh release create vX.Y.Z --title "..." --notes-file <notes>`，release note 按 Features / Internals / Fixes & housekeeping 分组，结尾附 `vPREV...vX.Y.Z` 的 compare 链接。

README 的 version / downloads / docs.rs badge 由 shields.io 实时拉取，发布后自动更新，无需手动改。
