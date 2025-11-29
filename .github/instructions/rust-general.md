# Rust 通用编程与架构设计指南

这份指南总结了适用于任意 Rust 项目的通用编程风格、设计模式和思维方式。

## 🧠 核心设计哲学 (Core Philosophy)

1.  **实用主义至上 (Pragmatism > Dogma)**
    *   不要为了追求“纯粹的 Rust 安全性”而过度设计。
    *   在应用层 (Application Layer)，为了架构的简洁性，允许使用 `unsafe`、单例 (Singletons) 或全局状态 (Global State)，前提是封装良好。
    *   **性能与开发效率**优于绝对的零成本抽象。

2.  **控制反转 (Inversion of Control)**
    *   倾向于**框架式设计**：库/框架控制主循环 (Main Loop) 和生命周期，用户代码通过实现 Trait (如 `OuterApp`, `Plugin`) 接入。
    *   避免用户代码手动管理复杂的生命周期顺序。

3.  **显式优于隐式 (Explicit is better than Implicit)**
    *   关键的初始化顺序、资源依赖关系应当在代码结构中体现（例如通过 `OnceCell` 显式延迟初始化）。

## 📝 代码风格 (Code Style)

### 1. 注释与文档
*   **语言**: 必须使用 **简体中文**。
*   **区块分隔**: 在长函数（如主循环、初始化流程）中，使用显式的分隔线注释来区分逻辑阶段。
    ```rust
    // Initialization ============================
    // ...
    // Update Logic ==============================
    // ...
    // Render / Output ===========================
    ```
*   **文档注释**: 公开的 API (`pub`) 必须包含 `///` 文档，说明“是什么”、“怎么用”以及“注意事项”。

### 2. 错误处理策略
*   **应用层 (Binaries/Examples)**:
    *   **Fail Fast**: 允许使用 `unwrap()` 或 `expect()`。如果初始化失败或核心资源缺失，直接崩溃比带病运行更好。
    *   使用自定义 `panic_handler` 记录日志。
*   **库层 (Libraries)**:
    *   必须返回 `Result`，严禁随意 panic。
    *   错误类型应当清晰明确 (使用 `thiserror` 或 `anyhow` 简化定义)。

### 3. 命名规范
*   遵循 Rust 标准 (`snake_case` 变量/函数, `PascalCase` 类型)。
*   **语义化命名**: 变量名应体现其生命周期或用途 (e.g., `last_render_area`, `init_flag`)。

## 🏗️ 通用设计模式 (Design Patterns)

### 1. 延迟初始化 (Lazy Initialization)
*   **场景**: 当某些资源的创建依赖于其他系统（如窗口句柄、上下文）已存在时。
*   **工具**: 频繁使用 `std::cell::OnceCell`, `std::sync::OnceLock` 或 `lazy_static`。
*   **优势**: 解耦了结构体的创建与初始化，避免了复杂的 `Option<T>` 检查（一旦初始化后即为确定的值）。

### 2. 上下文单例 (Context Singletons)
*   **场景**: 跨模块访问的核心系统（如日志、配置、设备上下文）。
*   **模式**: 使用全局静态变量 (Static) 或 `ThreadLocal` 存储核心上下文，提供 `Context::get()` 访问器。
*   **权衡**: 虽然破坏了纯函数式特性，但极大地简化了深层调用栈的参数传递。

### 3. 内部可变性 (Interior Mutability)
*   **场景**: 在不可变引用 (`&self`) 的接口中修改内部状态（如缓存、计时器、性能计数器）。
*   **工具**: `RefCell` (单线程), `RwLock`/`Mutex` (多线程)。
*   **原则**: 尽量将可变性限制在小范围内，对外暴露简洁的不可变 API。

### 4. 泛型应用入口 (Generic App Entry)
*   使用泛型结构体封装通用逻辑：
    ```rust
    struct AppRunner<T: UserLogic> {
        core_system: CoreSystem,
        user_logic: T, // 用户逻辑被包含在框架中
    }
    ```

## 🛠️ 工具与生态习惯

1.  **日志与监控**:
    *   项目必须集成 `log` 或 `tracing`。
    *   关键性能路径必须集成 Profiling 工具 (如 `tracy-client`)，并使用 `span!` 宏标记。

2.  **数学库**:
    *   图形/游戏相关项目首选 `glam`。

3.  **构建脚本**:
    *   善用 `build.rs` 处理非 Rust 依赖（C++ 库编译、资源处理、代码生成）。
