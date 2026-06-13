//! NekoSpice 可执行文件入口点。
//!
//! eframe + wgpu 需要大量栈空间用于 GPU 初始化。
//! 使用 `scripts/run.sh` 启动会自动设置足够的栈空间。
//! 手动启动时建议: `ulimit -s unlimited && cargo run -p nsp-app`

fn main() {
    if let Err(result) = nsp_app::run_native() {
        eprintln!("error: {result}");
        std::process::exit(1);
    }
}
