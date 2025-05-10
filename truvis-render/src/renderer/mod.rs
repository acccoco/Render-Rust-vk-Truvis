use crate::renderer::scene_manager::SceneManager;
use glam::Vec4Swizzles;
use model_manager::component::mesh::SimpleMesh;
use shader_binding::shader;
use std::collections::HashMap;
use std::iter::zip;
use std::rc::Rc;
use truvis_rhi::core::command_buffer::RhiCommandBuffer;

pub mod bindless;
pub mod frame_scene;
pub mod framebuffer;
pub mod scene_manager;
