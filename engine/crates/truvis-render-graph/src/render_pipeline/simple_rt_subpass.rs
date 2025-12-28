use crate::apis::render_pass::RenderSubpass;
use crate::graph::node::ImageNode;
use crate::render_context::RenderContext;
use ash::vk;
use itertools::Itertools;
use truvis_crate_tools::count_indexed_array;
use truvis_crate_tools::enumed_map;
use truvis_crate_tools::resource::TruvisPath;
use truvis_descriptor_layout_macro::DescriptorBinding;
use truvis_gfx::descriptors::descriptor::GfxDescriptorSetLayout;
use truvis_gfx::utilities::descriptor_cursor::GfxDescriptorCursor;
use truvis_gfx::{
    commands::{barrier::GfxImageBarrier, command_buffer::GfxCommandBuffer},
    gfx::Gfx,
    pipelines::shader::{GfxShaderGroupInfo, GfxShaderModuleCache, GfxShaderStageInfo},
    resources::special_buffers::sbt_buffer::GfxSBTBuffer,
};
use truvis_render_interface::global_descriptor_sets::GlobalDescriptorSets;
use truvis_render_interface::handles::{GfxImageHandle, GfxImageViewHandle};
use truvis_shader_binding::truvisl;

pub struct GfxRtPipeline {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
}
impl Drop for GfxRtPipeline {
    fn drop(&mut self) {
        let gfx_device = Gfx::get().gfx_device();
        unsafe {
            gfx_device.destroy_pipeline(self.pipeline, None);
            gfx_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}

enumed_map!(ShaderStage<GfxShaderStageInfo>: {
    RayGen: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::RAYGEN_KHR,
        entry_point: c"main_ray_gen",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
    SkyMiss: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::MISS_KHR,
        entry_point: c"sky_miss",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
    ShadowMiss: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::MISS_KHR,
        entry_point: c"shadow_miss",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
    ClosestHit: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::CLOSEST_HIT_KHR,
        entry_point: c"main_closest_hit",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
    TransAny: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::ANY_HIT_KHR,
        entry_point: c"trans_any",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
    DiffuseCall: GfxShaderStageInfo {
        stage: vk::ShaderStageFlags::CALLABLE_KHR,
        entry_point: c"diffuse_callable",
        path: TruvisPath::shader_build_path_str("rt/rt.slang"),
    },
});

enumed_map!(ShaderGroups<GfxShaderGroupInfo>: {
    RayGen: GfxShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::RayGen.index() as u32,
        ..GfxShaderGroupInfo::unused()
    },
    SkyMiss: GfxShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::SkyMiss.index() as u32,
        ..GfxShaderGroupInfo::unused()
    },
    ShadowMiss: GfxShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::ShadowMiss.index() as u32,
        ..GfxShaderGroupInfo::unused()
    },
    Hit: GfxShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
        closest_hit: ShaderStage::ClosestHit.index() as u32,
        any_hit: ShaderStage::TransAny.index() as u32,
        ..GfxShaderGroupInfo::unused()
    },
    DiffuseCall: GfxShaderGroupInfo {
        ty: vk::RayTracingShaderGroupTypeKHR::GENERAL,
        general: ShaderStage::DiffuseCall.index() as u32,
        ..GfxShaderGroupInfo::unused()
    },
});

pub struct SBTRegions {
    sbt_region_raygen: vk::StridedDeviceAddressRegionKHR,
    sbt_region_miss: vk::StridedDeviceAddressRegionKHR,
    sbt_region_hit: vk::StridedDeviceAddressRegionKHR,
    sbt_region_callable: vk::StridedDeviceAddressRegionKHR,

    _sbt_buffer: GfxSBTBuffer,
}
impl SBTRegions {
    const RAYGEN_SBT_REGION: usize = ShaderGroups::RayGen.index();
    const MISS_SBT_REGION: &'static [usize] = &[ShaderGroups::SkyMiss.index(), ShaderGroups::ShadowMiss.index()];
    const HIT_SBT_REGION: &'static [usize] = &[ShaderGroups::Hit.index()];
    const CALLABLE_SBT_REGION: &'static [usize] = &[ShaderGroups::DiffuseCall.index()];

    pub fn create_sbt(pipeline: &GfxRtPipeline) -> Self {
        let rt_pipeline_props = Gfx::get().rt_pipeline_props();

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

        let sbt_buffer = GfxSBTBuffer::new(
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
                Gfx::get()
                    .gfx_device()
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
        }

        Self {
            sbt_region_raygen,
            sbt_region_miss,
            sbt_region_hit,
            sbt_region_callable,
            _sbt_buffer: sbt_buffer,
        }
    }
}
impl Drop for SBTRegions {
    fn drop(&mut self) {
        log::info!("Destroy SBTRegions");
    }
}

/// pass 的依赖信息
pub struct SimpleRtPassDep {
    pub accum_image: ImageNode,
}
impl Default for SimpleRtPassDep {
    fn default() -> Self {
        Self {
            accum_image: ImageNode {
                stage: vk::PipelineStageFlags2::RAY_TRACING_SHADER_KHR,
                access: vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::SHADER_WRITE,
                layout: vk::ImageLayout::GENERAL,
            },
        }
    }
}

/// 传入 pass 的数据
pub struct SimpleRtPassData {
    pub accum_image: GfxImageHandle,
    pub accum_image_view: GfxImageViewHandle,
}

#[derive(DescriptorBinding)]
struct SimpleRtDescriptorBinding {
    #[binding = 0]
    #[descriptor_type = "ACCELERATION_STRUCTURE_KHR"]
    #[stage = "RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR"]
    #[count = 1]
    _tlas: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "RAYGEN_KHR | CLOSEST_HIT_KHR | ANY_HIT_KHR | CALLABLE_KHR | MISS_KHR"]
    #[count = 1]
    _rt_color: (),
}

pub struct SimpleRtSubpass {
    pipeline: GfxRtPipeline,
    _sbt: SBTRegions,
    _rt_descriptor_set_layout: GfxDescriptorSetLayout<SimpleRtDescriptorBinding>,
}
impl SimpleRtSubpass {
    pub fn new(render_descriptor_sets: &GlobalDescriptorSets) -> Self {
        let mut shader_module_cache = GfxShaderModuleCache::new();
        let stage_infos = ShaderStage::iter()
            .map(|stage| stage.value())
            .map(|stage| {
                vk::PipelineShaderStageCreateInfo::default()
                    .module(shader_module_cache.get_or_load(stage.path()).handle())
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
            .size(size_of::<truvisl::rt::PushConstants>() as u32);

        let rt_descriptor_set_layout = GfxDescriptorSetLayout::<SimpleRtDescriptorBinding>::new(
            vk::DescriptorSetLayoutCreateFlags::PUSH_DESCRIPTOR_KHR,
            "simple-rt-descriptor-set-layout",
        );

        let pipeline_layout = {
            let mut descriptor_set_layouts = render_descriptor_sets.global_set_layouts();
            descriptor_set_layouts.push(rt_descriptor_set_layout.handle());
            let pipeline_layout_ci = vk::PipelineLayoutCreateInfo::default()
                .set_layouts(&descriptor_set_layouts)
                .push_constant_ranges(std::slice::from_ref(&push_constant_range));

            unsafe { Gfx::get().gfx_device().create_pipeline_layout(&pipeline_layout_ci, None).unwrap() }
        };
        Gfx::get().gfx_device().set_object_debug_name(pipeline_layout, "simple-rt-pipeline-layout");
        let pipeline_ci = vk::RayTracingPipelineCreateInfoKHR::default()
            .stages(&stage_infos)
            .groups(&shader_groups)
            .layout(pipeline_layout)
            // 这个仅仅是用来分配栈内存的，并不会在超过递归深度后让调用被丢弃
            // 需要手动跟踪递归深度
            .max_pipeline_ray_recursion_depth(2);

        let pipeline = unsafe {
            Gfx::get()
                .gfx_device()
                .ray_tracing_pipeline()
                .create_ray_tracing_pipelines(
                    vk::DeferredOperationKHR::null(),
                    vk::PipelineCache::null(),
                    std::slice::from_ref(&pipeline_ci),
                    None,
                )
                .unwrap()[0]
        };
        Gfx::get().gfx_device().set_object_debug_name(pipeline, "simple-rt-pipeline");

        shader_module_cache.destroy();

        let rt_pipeline = GfxRtPipeline {
            pipeline,
            pipeline_layout,
        };
        let sbt = SBTRegions::create_sbt(&rt_pipeline);

        Self {
            pipeline: rt_pipeline,
            _sbt: sbt,
            _rt_descriptor_set_layout: rt_descriptor_set_layout,
        }
    }
    pub fn ray_trace(&self, render_context: &RenderContext, cmd: &GfxCommandBuffer, pass_data: SimpleRtPassData) {
        let frame_label = render_context.frame_counter.frame_label();

        let _rt_handle = render_context.bindless_manager.get_shader_uav_handle(pass_data.accum_image_view);
        let rt_image = render_context.gfx_resource_manager.get_image(pass_data.accum_image).unwrap().handle();
        let rt_image_view =
            render_context.gfx_resource_manager.get_image_view(pass_data.accum_image_view).unwrap().handle();

        cmd.begin_label("Ray trace", glam::vec4(0.0, 1.0, 0.0, 1.0));

        cmd.cmd_bind_pipeline(vk::PipelineBindPoint::RAY_TRACING_KHR, self.pipeline.pipeline);

        cmd.push_descriptor_set(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            self.pipeline.pipeline_layout,
            truvisl::RT_SET_NUM,
            &[
                SimpleRtDescriptorBinding::tlas().write_tals(
                    vk::DescriptorSet::null(),
                    0,
                    vec![render_context.gpu_scene.tlas(frame_label).unwrap().handle()],
                ),
                SimpleRtDescriptorBinding::rt_color().write_image(
                    vk::DescriptorSet::null(),
                    0,
                    vec![
                        vk::DescriptorImageInfo::default()
                            .image_layout(vk::ImageLayout::GENERAL)
                            .image_view(rt_image_view),
                    ],
                ),
            ],
        );

        cmd.bind_descriptor_sets(
            vk::PipelineBindPoint::RAY_TRACING_KHR,
            self.pipeline.pipeline_layout,
            0,
            &render_context.global_descriptor_sets.global_sets(frame_label),
            None,
        );
        let spp = 4;
        let mut push_constant = truvisl::rt::PushConstants {
            spp,
            spp_idx: 0,
            channel: render_context.pipeline_settings.channel,
            _padding_1: 0,
        };
        for spp_idx in 0..spp {
            push_constant.spp_idx = spp_idx;

            if spp_idx != 0 {
                cmd.image_memory_barrier(
                    vk::DependencyFlags::empty(),
                    &[GfxImageBarrier::new()
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
                    render_context.frame_settings.frame_extent.width,
                    render_context.frame_settings.frame_extent.height,
                    1,
                ],
            );
        }

        cmd.end_label();
    }
}
impl RenderSubpass for SimpleRtSubpass {}
impl Drop for SimpleRtSubpass {
    fn drop(&mut self) {
        log::info!("Destroy SimlpeRtPass");
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
