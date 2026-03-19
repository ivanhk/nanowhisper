完全重置并重新安装 NanoWhisper，按顺序执行以下步骤：

1. 杀掉所有 NanoWhisper 相关进程（pkill -f NanoWhisper，忽略报错）
2. 删除 /Applications/NanoWhisper.app（如果存在）
3. 删除用户配置目录 ~/.nanowhisper/（settings.json、history.db、audio/）
4. 重置 macOS 系统权限：
   - tccutil reset Accessibility com.nanowhisper.app
   - tccutil reset Microphone com.nanowhisper.app
5. 重新编译：npm run tauri build
6. 将构建产物安装到 /Applications/（从 src-tauri/target/release/bundle/macos/ 复制 NanoWhisper.app）
7. 启动应用：open /Applications/NanoWhisper.app

每一步执行前先告知我在做什么，遇到错误时停下来询问。
