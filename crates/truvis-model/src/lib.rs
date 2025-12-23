//! 模型和顶点数据管理
//!
//! 提供顶点数据结构定义、几何体创建、模型加载等功能。
//! 通过 `truvis-cxx` 集成 Assimp 库实现多格式模型加载。
//!
//! # 顶点布局
//! - `VertexLayoutAoSPosColor`: Position + Color (AoS 布局)
//! - `VertexLayoutAoSPosNorTexCoord`: Position + Normal + TexCoord
//!
//! # 使用示例
//! ```ignore
//! use truvis_model::vertex::aos_pos_color::VertexLayoutAoSPosColor;
//! use truvis_model::components::geometry::Geometry;
//!
//! // 创建内置几何体（自动上传 GPU）
//! let triangle: Geometry<VertexLayoutAoSPosColor> = VertexLayoutAoSPosColor::triangle();
//! ```

pub mod components;
pub mod guid_new_type;
pub mod shapes;
pub mod vertex;
