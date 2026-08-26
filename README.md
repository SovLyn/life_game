# Life Game（粒子生命）

基于 [wgpu](https://wgpu.rs) 计算着色器的粒子生命（Particle Life）模拟，使用 [egui](https://egui.rs) / eframe 构建界面，通过 [Trunk](https://trunkrs.dev) 编译为 WebAssembly，在浏览器中运行。

## 运行

需要 Rust 工具链和 Trunk：

```sh
cargo install trunk
```

本地开发（默认监听 `http://localhost:8080`）：

```sh
trunk serve
```

发布构建（产物输出到 `dist/`）：

```sh
trunk build --release
```

## 特性

- 每帧 5 个 WebGPU 计算着色器 pass 完成模拟
- 均匀网格（uniform grid）空间划分加速，复杂度 O(n·k) 而非全对全的 O(n²)
- egui 面板实时调节参数：相互作用矩阵、作用范围、透明度、粒子数量、颜色等
- 配置自动持久化（localStorage）

## License

[MIT](LICENSE)