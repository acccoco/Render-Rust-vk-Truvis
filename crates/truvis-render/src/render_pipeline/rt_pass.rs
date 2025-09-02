use std::{cell::RefCell, rc::Rc};

use ash::vk;
use itertools::Itertools;
use shader_binding::{shader, shader::ImageHandle};
use truvis_crate_tools::{const_map, count_indexed_array, resource::TruvisPath};
use truvis_rhi::{
    commands::{barrier::ImageBarrier, command_buffer::CommandBuffer},
    foundation::device::DeviceFunctions,
    pipelines::shader::{ShaderGroupInfo, ShaderModule, ShaderStageInfo},
    render_context::RenderContext,
    resources::special_buffers::{sbt_buffer::SBTBuffer, structured_buffer::StructuredBuffer},
};

use crate::{
    pipeline_settings::{FrameSettings, PipelineSettings},
    renderer::{bindless::BindlessManager, frame_controller::FrameController, gpu_scene::GpuScene},
};

pub struct RhiRtPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,

    pub device_functions: Rc<DeviceFunctions>,
}
impl Drop for RhiRtPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device_functions.destroy_pipeline(self.pipeline, None);
            self.device_functions.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

const_map!(ShaderStage<ShaderStageInfo>: {
    RayGen: ShaderStageInfo {
        stage: vk::ShaderStageFlags::RAYGEN_KHR,
        entry_point: cstr::cstr!("main_ray_gen"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
    SkyMiss: ShaderStageInfo {
        stage: vk::ShaderStageFlags::MISS_KHR,
        entry_point: cstr::cstr!("sky_miss"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
    ShadowMiss: ShaderStageInfo {
        stage: vk::ShaderStageFlags::MISS_KHR,
        entry_point: cstr::cstr!("shadow_miss"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
    ClosestHit: ShaderStageInfo {
        stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        entry_point: cstr::cstr!("main_closest_hit"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
    TransAny: ShaderStageInfo {
        stage: vk::ShaderStageFlags::ANY_HIT_KHR,
        entry_point: cstr::cstr!("trans_any"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
    DiffuseCall: ShaderStageInfo {
        stage: vk::ShaderStageFlags::CALLABLE_KHR,
        entry_point: cstr::cstr!("diffuse_callable"),
        path: TruvisPath::shader_path("rt/rt.slang.spv"),
    },
});

const_map!(ShaderGroups<ShaderGroupInfo>: {
    RayGen: ShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::RayGen.index() as u32,
        ..ShaderGroupInfo::unused()
    },
    SkyMiss: ShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::SkyMiss.index() as u32,
        ..ShaderGroupInfo::unused()
    },
    ShadowMiss: ShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::ShadowMiss.index() as u32,
        ..ShaderGroupInfo::unused()
    },
    Hit: ShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
        closest_hit: ShaderStage::ClosestHit.index() as u32,
        any_hit: ShaderStage::TransAny.index() as u32,
        ..ShaderGroupInfo::unused()
    },
    DiffuseCall: ShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::DiffuseCall.index() as u32,
        ..ShaderGroupInfo::unused()
    },
});

pub struct SBTRegions {
    sbt_region_raygen: vk::StridedDeviceAddressRegionKHR,
    sbt_region_miss: vk::StridedDeviceAddressRegionKHR,
    sbt_region_hit: vk::StridedDeviceAddressRegionKHR,
    sbt_region_callable: vk::StridedDeviceAddressRegionKHR,

    _sbt_buffer: SBTBuffer,

    device_functions: Rc<DeviceFunctions>,
}
impl SBTRegions {
    const RAYGEN_SBT_REGION: usize = ShaderGroups::RayGen.index();
    const MISS_SBT_REGION: &'static [usize] = &[ShaderGroups::SkyMiss.index(), ShaderGroups::ShadowMiss.index()];
    const HIT_SBT_REGION: &'static [usize] = &[ShaderGroups::Hit.index()];
    const CALLABLE_SBT_REGION: &'static [usize] = &[ShaderGroups::DiffuseCall.index()];

    pub fn create_sbt(render_context: &RenderContext, pipeline: &RhiRtPipeline) -> Self {
        let rt_pipeline_props = render_context.rt_pipeline_props();

        // 因为不需要 user data，所以可以直接使用 shader group handle size
        let aligned_shader_group_handle_size = helper::align_up(
            rt_pipeline_props.shader_group_handle_size,
            rt_pipeline_props.shader_group_handle_alignment,
        );

        // 每一个 region 需要使用 base align 进行对齐
        let raygen_shader_group_region_size =
            helper::align_up(aligned_shader_group_handle_size, rt_pipeline_props.shader_group_base_alignment);
        let miss_shader_group_region_size = helper::align_up(
            Self::MISS_SBT_REGION.len() as u32 * aligned_shader_group_handle_size,
            rt_pipeline_props.shader_group_base_alignment,
        );
        let hit_shader_group_region_size = helper::align_up(
            Self::HIT_SBT_REGION.len() as u32 * aligned_shader_group_handle_size,
            rt_pipeline_props.shader_group_base_alignment,
        );
        let callable_shader_group_region_size = helper::align_up(
            Self::CALLABLE_SBT_REGION.len() as u32 * aligned_shader_group_handle_size,
            rt_pipeline_props.shader_group_base_alignment,
        );

        let mut sbt_buffer = SBTBuffer::new(
            render_context.device_functions(),
            render_context.allocator(),
            (raygen_shader_group_region_size
                + miss_shader_group_region_size
                + hit_shader_group_region_size
                + callable_shader_group_region_size) as vk::DeviceSize,
            rt_pipeline_props.shader_group_base_alignment as vk::DeviceSize,
            "simple-rt-sbt",
        );

        // 找到每个 shader group 在 SBT 中的地址
        let sbt_address = sbt_buffer.device_address();

        let sbt_region_raygen = vk::StridedDeviceAddressRegionKHR::default()
            .stride(raygen_shader_group_region_size as vk::DeviceSize) // raygen 的 stride 需要和 size 一样
            .size(raygen_shader_group_region_size as vk::DeviceSize)
            .device_address(sbt_address);
        let sbt_region_miss = vk::StridedDeviceAddressRegionKHR::default()
            .stride(aligned_shader_group_handle_size as vk::DeviceSize)
            .size(miss_shader_group_region_size as vk::DeviceSize)
            .device_address(sbt_address + raygen_shader_group_region_size as vk::DeviceSize);
        let sbt_region_hit = vk::StridedDeviceAddressRegionKHR::default()
            .stride(aligned_shader_group_handle_size as vk::DeviceSize)
            .size(hit_shader_group_region_size as vk::DeviceSize)
            .device_address(
                sbt_address
                    + raygen_shader_group_region_size as vk::DeviceSize
                    + miss_shader_group_region_size as vk::DeviceSize,
            );
        let sbt_region_callable = vk::StridedDeviceAddressRegionKHR::default()
            .stride(aligned_shader_group_handle_size as vk::DeviceSize)
            .size(callable_shader_group_region_size as vk::DeviceSize)
            .device_address(
                sbt_address
                    + raygen_shader_group_region_size as vk::DeviceSize
                    + miss_shader_group_region_size as vk::DeviceSize
                    + hit_shader_group_region_size as vk::DeviceSize,
            );

        // 从 pipeline 中获取 shader 的 handle，并且将 shader handle 写入到 shader
        // binding table 中
        {
            let shader_group_handle_data = unsafe {
                render_context
                    .device_functions()
                    .ray_tracing_pipeline()
                    .get_ray_tracing_shader_group_handles(
                        pipeline.pipeline,
                        0,
                        ShaderGroups::COUNT as u32,
                        (ShaderGroups::COUNT as u32 * rt_pipeline_props.shader_group_handle_size) as usize,
                    )
                    .unwrap()
            };

            let copy_shader_group_hande = |group_handle_idx: usize, sbt_handle_host_addr: *mut u8| unsafe {
                let start_bytes = rt_pipeline_props.shader_group_handle_size as usize * group_handle_idx;
                let length_bytes = rt_pipeline_props.shader_group_handle_size as usize;
                let src = &shader_group_handle_data[start_bytes..start_bytes + length_bytes];

                let dst = std::slice::from_raw_parts_mut(
                    sbt_handle_host_addr,
                    rt_pipeline_props.shader_group_handle_size as usize,
                );
                dst.copy_from_slice(src);
            };

            let sbt_buffer_size = sbt_buffer.size();
            sbt_buffer.map();
            let sbt_host_address = sbt_buffer.mapped_ptr();

            let sbt_host_addr_raygen = sbt_host_address;
            copy_shader_group_hande(Self::RAYGEN_SBT_REGION, sbt_host_address);

            let sbt_host_addr_miss = sbt_host_addr_raygen.wrapping_byte_add(sbt_region_raygen.size as usize);
            for (idx, group_handle_idx) in Self::MISS_SBT_REGION.iter().enumerate() {
                copy_shader_group_hande(
                    *group_handle_idx,
                    sbt_host_addr_miss.wrapping_byte_add(idx * sbt_region_miss.stride as usize),
                );
            }

            let sbt_host_addr_hit = sbt_host_addr_miss.wrapping_byte_add(sbt_region_miss.size as usize);
            for (idx, group_handle_idx) in Self::HIT_SBT_REGION.iter().enumerate() {
                copy_shader_group_hande(
                    *group_handle_idx,
                    sbt_host_addr_hit.wrapping_byte_add(idx * sbt_region_hit.stride as usize),
                );
            }

            let sbt_host_addr_callable = sbt_host_addr_hit.wrapping_byte_add(sbt_region_hit.size as usize);
            for (idx, group_handle_idx) in Self::CALLABLE_SBT_REGION.iter().enumerate() {
                copy_shader_group_hande(
                    *group_handle_idx,
                    sbt_host_addr_callable.wrapping_byte_add(idx * sbt_region_callable.stride as usize),
                );
            }

            sbt_buffer.flush(0, sbt_buffer_size);
            sbt_buffer.unmap()
        }

        Self {
            sbt_region_raygen,
            sbt_region_miss,
            sbt_region_hit,
            sbt_region_callable,
            _sbt_buffer: sbt_buffer,
            device_functions: render_context.device_functions().clone(),
        }
    }
}

pub struct SimlpeRtPass {
    pipeline: RhiRtPipeline,
    _bindless_mgr: Rc<RefCell<BindlessManager>>,

    _sbt: SBTRegions,

    device_functions: Rc<DeviceFunctions>,
}
impl SimlpeRtPass {
    pub fn new(render_context: &RenderContext, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let shader_modules = ShaderStage::iter()
            .map(|stage| stage.value())
            .map(|stage| ShaderModule::new(render_context.device_functions(), stage.path()))
            .collect_vec();
        let stage_infos = ShaderStage::iter()
            .map(|stage| stage.value())
            .zip(shader_modules.iter())
            .map(|(stage, shader_modele)| {
                vk::PipelineShaderStageCreateInfo::default()
                    .module(shader_modele.handle())
                    .stage(stage.stage)
                    .name(stage.entry_point)
            })
            .collect_vec();

        let shader_groups = ShaderGroups::iter()
            .map(|group| group.value())
            .map(|group| vk::RayTracingShaderGroupCreateInfoKHR {
                ty: group.ty,
                general_shader: group.general,
                any_hit_shader: group.any_hit,
                closest_hit_shader: group.closest_hit,
                intersection_shader: group.intersection,
                ..Default::default()
            })
            .collect_vec();

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(
                vk::ShaderStageFlags::RAYGEN_KHR
                    | vk::ShaderStageFlags::MISS_KHR
                    | vk::ShaderStageFlags::ANY_HIT_KHR
                    | vk::ShaderStageFlags::CALLABLE_KHR
                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            )
            .offset(0)
            .size(size_of::<shader::rt::PushConstants>() as u32);

        let pipeline_layout = {
            let bineless_mgr = bindless_mgr.borrow();

            let descriptor_sets = [bineless_mgr.bindless_descriptor_layout.handle()];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_sets)
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));

            unsafe { render_context.device_functions().create_pipeline_layout(&pipeline_layout_ci, None).unwrap() }
        };
        let pipeline_ci = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&stage_infos)
            .groups(&shader_groups)
            .layout(pipeline_layout)
            // 这个仅仅是用来分配栈内存的，并不会在超过递归深度后让调用被丢弃
            // 需要手动跟踪递归深度
            .max_pipeline_ray_recursion_depth(2);

        let pipeline = unsafe {
            render_context
                .device_functions()
                .ray_tracing_pipeline()
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_ci),
                    None,
                )
                .unwrap()[0]
        };

        shader_modules.into_iter().for_each(|module| module.destroy());

        let rt_pipeline = RhiRtPipeline {
            pipeline,
            pipeline_layout,
            device_functions: render_context.device_functions().clone(),
        };
        let sbt = SBTRegions::create_sbt(render_context, &rt_pipeline);

        Self {
            pipeline: rt_pipeline,
            _sbt: sbt,
            _bindless_mgr: bindless_mgr,
            device_functions: render_context.device_functions().clone(),
        }
    }
    pub fn ray_trace(
        &self,
        cmd: &CommandBuffer,
        frame_ctrl: &FrameController,
        framse_settings: &FrameSettings,
        pipeline_settings: &PipelineSettings,
        rt_image: vk::Image,
        rt_handle: ImageHandle,
        per_frame_data: &StructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
    ) {
        let frame_label = frame_ctrl.frame_label();

        cmd.begin_label("Ray trace", glam::vec4(0.0, 1.0, 0.0, 1.0));

        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::RAY_TRACING_KHR, self.pipeline.pipeline);
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            self.pipeline.pipeline_layout,
            0,
            &[self._bindless_mgr.borrow().bindless_descriptor_sets[*frame_label].handle()],
            None,
        );
        let spp = 4;
        let mut push_constant = shader::rt::PushConstants {
            frame_data: per_frame_data.device_address(),
            scene: gpu_scene.scene_device_address(frame_label),
            rt_render_target: rt_handle,
            spp,
            spp_idx: 0,
            channel: pipeline_settings.channel,
        };
        for spp_idx in 0..spp {
            push_constant.spp_idx = spp_idx;

            if spp_idx != 0 {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[ImageBarrier::new()
                        .image(rt_image)
                        .image_aspect_flag(vk::ImageAspectFlags::COLOR)
                        .src_mask(
                            vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                            vk::AccessFlags2::SHADER_WRITE | vk::AccessFlags2::SHADER_READ,
                        )
                        .dst_mask(
                            vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                            vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                        )],
                );
            }

            cmd.cmd_push_constants(
                self.pipeline.pipeline_layout,
                vk::ShaderStageFlags::RAYGEN_KHR
                    | vk::ShaderStageFlags::MISS_KHR
                    | vk::ShaderStageFlags::ANY_HIT_KHR
                    | vk::ShaderStageFlags::CALLABLE_KHR
                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                0,
                bytemuck::bytes_of(&push_constant),
            );

            cmd.trace_rays(
                &self._sbt.sbt_region_raygen,
                &self._sbt.sbt_region_miss,
                &self._sbt.sbt_region_hit,
                &self._sbt.sbt_region_callable,
                [
                    framse_settings.frame_extent.width,
                    framse_settings.frame_extent.height,
                    1,
                ],
            );
        }

        cmd.end_label();
    }
}

mod helper {
    /// round x up to a multiple of align
    ///
    /// * align must be a power of 2
    pub fn align_up(x: u32, align: u32) -> u32 {
        (x + (align - 1)) & !(align - 1)
    }
}
