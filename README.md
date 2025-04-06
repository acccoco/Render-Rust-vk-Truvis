[![Lines of Code](https://tokei.rs/b1/github/acccoco/Render-Rust-vk-Truvis)](https://github.com/acccoco/Render-Rust-vk-Truvis)
[![CI Status](https://github.com/acccoco/Render-Rust-vk-Truvis/workflows/Rust/badge.svg)](https://github.com/acccoco/Render-Rust-vk-Truvis/actions)

# TODO

- [ ] 多种材质系统，多种渲染流程(forward, deferred, etc)
- [ ] 使用 `hlsl` 而不是 `glsl`
- [x] 完善窗口系统，以及 `imgui`
    - [ ] `imgui` 支持图片 Texture
    - [ ] `winit` 注册事件回调，而不是主动调用 `render_loop()`
- [x] (optional)在 app 内配置 `vulkan` 的各种 `layer` 参数
- [x] 不要 static，减少函数理解的心智负担
- [x] 不要 option，减少调用开销。
- [ ] 支持 窗口 resize

`Texture` 的实现思路：`texture` 应该包含如下内容：

* `image`
* `image view`
* `descriptor image info`
* `sampler`

# 设计原则

不应该使用 Rust 的生命周期和引用跟踪来确保 GPU 资源的合法性，因为 handle 之类的本就是 GPU 上的资源。

借助 Rust 反而会减少灵活性，引入不必要的开销。

尽量保证 Handle 都是可 copy 的就好

# Debug 命名规范

* object name：`[frame-A-id][pass]name`
* queue label/cmd label：`[frame-A-id][pass]name`

# 坐标系

* model space: Right-Handed, Y-Up
* world: Right-Handed, Y-Up
* camera: 右手，Y-Up。相机朝向 -Z
* clip：x 和相机保持一直，y 和相机相反（朝下）
* framebuffer：顶点在左上角。深度 0-1，0 在最远处，1 在最近处

![坐标系](doc/img/coords.png)

注：背面剔除的时机：基于 framebuffer 中的三角形的顶点顺序。

已知 Blender 的坐标系是：Right-Handed, Z-Up

从当前应用到 Blender，需要：

```shell  
x_blender = x
y_blender = -z
z_blender = y
```