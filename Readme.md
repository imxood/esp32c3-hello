# 编译环境

### 安装 esp 工具链

cargo install espup

espup install

### 安装 esp idf 环境 (可选)

https://dl.espressif.com/dl/esp-idf/

我选择的版本是 v4.4

## 编译

如果安装了上一步的环境, 使用这个工具 打开终端, 会自动配置好 idf环境:

![](images/Readme/20230223214417.png)

如果没有这个 idf环境, 就会自动下载完整的 idf环境 到项目根路径的 .embuild目录

cd 到该项目路径

cargo run
