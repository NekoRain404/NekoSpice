//! NekoSpice 可执行文件入口点。
//!
//! NekoSpiceApp 的 Default 实现涉及大量文件 I/O 和原理图解析，
//! 加上 wgpu GPU 初始化，主线程需要充足的栈空间。
//! 使用 `scripts/run.sh` 启动会自动设置足够的栈空间。
//! 手动启动时: `ulimit -s unlimited && cargo run -p nsp-app`

fn main() {
    if let Err(result) = nsp_app::run_native() {
        eprintln!("error: {result}");
        std::process::exit(1);
    }
}
