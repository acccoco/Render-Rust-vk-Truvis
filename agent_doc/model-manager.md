# model-manager

## 概述
基础的模型管理 crate，提供 3D 模型的基础定义、顶点数据结构和几何体管理。作为渲染引擎的几何数据层。

## 架构组织

### 组件系统 (`src/component.rs`)
具体文件内容：
- **`DrsGeometry<V>`**: 顶点和索引缓冲区的封装
- **`DrsMaterial`**: 材质属性定义（PBR 参数）
- **光线追踪支持**: BLAS 创建和管理
- **GPU 资源管理**: 与 `truvis-rhi` 集成的缓冲区

### GUID 系统 (`src/guid_new_type.rs`)
- 全局唯一标识符的 new type 包装
- 资源和对象的唯一标识管理（`MatGuid`、`MeshGuid`）
- 类型安全的 ID 系统

### 顶点系统 (`src/vertex/`)
专门的顶点数据结构定义：
- **`vertex_pc.rs`**: 位置+颜色顶点格式（`VertexPosColor`）
- **`vertex_pnu.rs`**: 位置+法线+UV 顶点格式
- **`vertex_3d.rs`**: 完整 3D 顶点格式（`Vertex3D`）
- **`mod.rs`**: `VertexLayout` trait 定义，用于 Vulkan 集成

## 核心数据结构

### 几何体定义（来自 `src/component.rs`）
```rust
// 文件：crates/model-manager/src/component.rs
pub struct DrsGeometry<V: bytemuck::Pod> {
    pub vertex_buffer: RhiVertexBuffer<V>,
    pub index_buffer: RhiIndexBuffer,
}
pub type DrsGeometry3D = DrsGeometry<Vertex3D>;
```

### 材质定义（来自 `src/component.rs`）
```rust
// 文件：crates/model-manager/src/component.rs
#[derive(Default)]
pub struct DrsMaterial {
    pub base_color: glam::Vec4,
    pub emissive: glam::Vec4,
    pub metallic: f32,
    pub roughness: f32,
    pub opaque: f32,
    pub diffuse_map: String,
    pub normal_map: String,
}
```

### 顶点布局 Trait（来自 `src/vertex/mod.rs`）
```rust
// 文件：crates/model-manager/src/vertex/mod.rs
pub trait VertexLayout {
    fn vertex_input_bindings() -> Vec<vk::VertexInputBindingDescription>;
    fn vertex_input_attributes() -> Vec<vk::VertexInputAttributeDescription>;
}
```

## 特性支持

### bytemuck 集成
- 所有顶点类型都支持 `bytemuck` trait
- 安全的字节转换，直接上传到 GPU
- 零拷贝的数据传输

### UUID 支持
- 所有资源都有唯一标识符
- 资源的引用和查找
- 序列化和反序列化支持

### 数学库集成
- 使用 `glam` 进行向量和矩阵运算
- 与渲染管线的数学类型统一
- 高性能的 SIMD 优化

## 使用模式

### 创建几何体
```rust
let vertices = vec![
    Vertex {
        position: Vec3::new(-1.0, -1.0, 0.0),
        normal: Vec3::Z,
        tex_coord: Vec2::new(0.0, 0.0),
        // ...
    },
    // ... 更多顶点
];

let geometry = Geometry::new(vertices, indices);
```

### 组件化对象
```rust
let entity = Entity::new()
    .with_component(TransformComponent::default())
    .with_component(GeometryComponent::new(geometry))
    .with_component(MaterialComponent::new(material));
```

## 与其他 crate 的集成

### truvis-rhi 集成
- 顶点布局与 Vulkan 输入装配器对接
- 缓冲区创建的便捷方法
- GPU 资源的生命周期管理

### truvis-cxx 集成
- Assimp 加载的模型数据转换
- 外部格式到内部表示的转换
- 材质和纹理信息的提取

## 设计原则
- **简单性**: 专注于基础几何数据的表示
- **性能**: 优化的内存布局和 GPU 传输
- **扩展性**: 支持多种顶点格式和渲染需求
- **类型安全**: 强类型的资源标识和组件系统

## 发展方向
- 更多的顶点格式支持
- 高级几何操作（合并、细分等）
- 空间数据结构集成
- 动画和骨骼系统支持
