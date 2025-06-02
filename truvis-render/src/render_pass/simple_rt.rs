use crate::render_context::{FrameSettings, RenderContext};
use crate::renderer::bindless::BindlessManager;
use crate::renderer::gpu_scene::GpuScene;
use ash::vk;
use itertools::Itertools;
use shader_binding::shader;
use std::cell::RefCell;
use std::rc::Rc;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::create_named_array;
use truvis_rhi::core::buffer::{RhiSBTBuffer, RhiStructuredBuffer};
use truvis_rhi::core::command_buffer::RhiCommandBuffer;
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::shader::{RhiShaderModule, RhiShaderStageInfo, ShaderGroupInfo};
use truvis_rhi::rhi::Rhi;

pub struct RhiRtPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,

    pub device: Rc<RhiDevice>,
}
impl Drop for RhiRtPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

create_named_array!(
    ShaderStage,
    SHADER_STAGES,
    RhiShaderStageInfo,
    [
        (
            RayGen,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::RAYGEN_KHR,
                entry_point: cstr::cstr!("main_ray_gen"),
                path: "shader/build/rt/rt.slang.spv",
            }
        ),
        (
            SkyMiss,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::MISS_KHR,
                entry_point: cstr::cstr!("sky_miss"),
                path: "shader/build/rt/rt.slang.spv",
            }
        ),
        (
            ShadowMiss,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::MISS_KHR,
                entry_point: cstr::cstr!("shadow_miss"),
                path: "shader/build/rt/rt.slang.spv",
            }
        ),
        (
            ClosestHit,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
                entry_point: cstr::cstr!("main_closest_hit"),
                path: "shader/build/rt/rt.slang.spv",
            }
        ),
        (
            TransAny,
            RhiShaderStageInfo {
                stage: vk::ShaderStageFlags::ANY_HIT_KHR,
                entry_point: cstr::cstr!("trans_any"),
                path: "shader/build/rt/rt.slang.spv",
            }
        ),
    ]
);

create_named_array!(
    ShaderGroups,
    SHADER_GROUPS,
    ShaderGroupInfo,
    [
        (
            RayGen,
            ShaderGroupInfo {
                ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
                general: ShaderStage::RayGen.index() as u32,
                ..ShaderGroupInfo::unused()
            }
        ),
        (
            SkyMiss,
            ShaderGroupInfo {
                ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
                general: ShaderStage::SkyMiss.index() as u32,
                ..ShaderGroupInfo::unused()
            }
        ),
        (
            ShadowMiss,
            ShaderGroupInfo {
                ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
                general: ShaderStage::ShadowMiss.index() as u32,
                ..ShaderGroupInfo::unused()
            }
        ),
        (
            Hit,
            ShaderGroupInfo {
                ty: vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
                closest_hit: ShaderStage::ClosestHit.index() as u32,
                any_hit: ShaderStage::TransAny.index() as u32,
                ..ShaderGroupInfo::unused()
            }
        ),
    ]
);

pub struct SBTRegions {
    // TODO 这个字段不应该放在这里，这个是和场景有关的
    sbt_region_raygen: vk::StridedDeviceAddressRegionKHR,
    sbt_region_miss: vk::StridedDeviceAddressRegionKHR,
    sbt_region_hit: vk::StridedDeviceAddressRegionKHR,
    sbt_region_callable: vk::StridedDeviceAddressRegionKHR,

    _sbt_buffer: RhiSBTBuffer,

    _device: Rc<RhiDevice>,
}
impl SBTRegions {
    const RAYGEN_SBT_REGION: usize = ShaderGroups::RayGen.index();
    const MISS_SBT_REGION: &'static [usize] = &[ShaderGroups::SkyMiss.index(), ShaderGroups::ShadowMiss.index()];
    const HIT_SBT_REGION: &'static [usize] = &[ShaderGroups::Hit.index()];

    pub fn create_sbt(rhi: &Rhi, pipeline: &RhiRtPipeline) -> Self {
        let rt_pipeline_props = rhi.device.rt_pipeline_props();

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

        let mut sbt_buffer = RhiSBTBuffer::new(
            rhi,
            (raygen_shader_group_region_size + miss_shader_group_region_size + hit_shader_group_region_size)
                as vk::DeviceSize,
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

        // 从 pipeline 中获取 shader 的 handle，并且将 shader handle 写入到 shader binding table 中
        {
            let shader_group_handle_data = unsafe {
                rhi.device
                    .rt_pipeline_pf()
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

            sbt_buffer.flush(0, sbt_buffer_size);
            sbt_buffer.unmap()
        }

        Self {
            sbt_region_raygen,
            sbt_region_miss,
            sbt_region_hit,
            sbt_region_callable: vk::StridedDeviceAddressRegionKHR::default(),
            _sbt_buffer: sbt_buffer,
            _device: rhi.device.clone(),
        }
    }
}

pub struct SimlpeRtPass {
    pipeline: RhiRtPipeline,
    _bindless_mgr: Rc<RefCell<BindlessManager>>,

    _sbt: SBTRegions,

    _device: Rc<RhiDevice>,
}
impl SimlpeRtPass {
    pub fn new(rhi: &Rhi, bindless_mgr: Rc<RefCell<BindlessManager>>) -> Self {
        let shader_modules = ShaderStage::iter()
            .map(|stage| stage.value())
            .map(|stage| RhiShaderModule::new(rhi.device.clone(), stage.path()))
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
                    | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
            )
            .offset(0)
            .size(size_of::<shader::rt::PushConstants>() as u32);

        let pipeline_layout = {
            let bineless_mgr = bindless_mgr.borrow();

            let descriptor_sets = [bineless_mgr.bindless_layout.handle()];
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_sets)
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));

            unsafe { rhi.device.create_pipeline_layout(&pipeline_layout_ci, None).unwrap() }
        };
        let pipeline_ci = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&stage_infos)
            .groups(&shader_groups)
            .layout(pipeline_layout)
            // 这个仅仅是用来分配栈内存的，并不会在超过递归深度后让调用被丢弃
            // 需要手动跟踪递归深度
            .max_pipeline_ray_recursion_depth(2);

        let pipeline = unsafe {
            rhi.device
                .rt_pipeline_pf()
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
            device: rhi.device.clone(),
        };
        let sbt = SBTRegions::create_sbt(rhi, &rt_pipeline);

        Self {
            pipeline: rt_pipeline,
            _sbt: sbt,
            _bindless_mgr: bindless_mgr,
            _device: rhi.device.clone(),
        }
    }
    pub fn ray_trace(
        &self,
        cmd: &RhiCommandBuffer,
        render_ctx: &RenderContext,
        framse_settings: &FrameSettings,
        per_frame_data: &RhiStructuredBuffer<shader::PerFrameData>,
        gpu_scene: &GpuScene,
    ) {
        let frame_label = render_ctx.current_frame_label();

        cmd.begin_label("Ray trace", glam::vec4(0.0, 1.0, 0.0, 1.0));

        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::RAY_TRACING_KHR, self.pipeline.pipeline);
        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            self.pipeline.pipeline_layout,
            0,
            &[self._bindless_mgr.borrow().bindless_sets[frame_label].handle()],
            None,
        );
        let push_constant = shader::rt::PushConstants {
            frame_data: per_frame_data.device_address(),
            scene: gpu_scene.scene_device_address(frame_label),

            instance_idx: 0, // RT 不需要这个
            submesh_idx: 0,  // RT 不需要这个

            _padding_1: Default::default(),
            _padding_2: Default::default(),
        };
        cmd.cmd_push_constants(
            self.pipeline.pipeline_layout,
            vk::ShaderStageFlags::RAYGEN_KHR
                | vk::ShaderStageFlags::MISS_KHR
                | vk::ShaderStageFlags::ANY_HIT_KHR
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
                framse_settings.rt_rect.extent.width,
                framse_settings.rt_rect.extent.height,
                1,
            ],
        );

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
