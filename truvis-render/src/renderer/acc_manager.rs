use ash::vk;
use shader_layout_macro::ShaderLayout;
use std::rc::Rc;
use truvis_rhi::core::acceleration::RhiAcceleration;
use truvis_rhi::core::descriptor::{RhiDescriptorSet, RhiDescriptorSetLayout};
use truvis_rhi::core::device::RhiDevice;
use truvis_rhi::core::image::RhiImage2DView;
use truvis_rhi::rhi::Rhi;
use truvis_rhi::shader_cursor::ShaderCursor;

/// 管理加速结构之类的
pub struct AccManager {
    pub rt_descriptor_set_layout: RhiDescriptorSetLayout<RtBindings>,
    pub rt_descriptor_sets: Vec<RhiDescriptorSet<RtBindings>>,

    device: Rc<RhiDevice>,
}
impl AccManager {
    pub fn new(rhi: &Rhi, frames_in_flight: usize) -> Self {
        log::info!("Creating AccManager");

        // 创建光追专用的 descriptor set layout
        let rt_descriptor_set_layout = RhiDescriptorSetLayout::<RtBindings>::new(
            rhi,
            vk::DescriptorSetLayoutCreateFlags::empty(),
            "RtDescriptorSetLayout",
        );

        // 创建光追专用的 descriptor sets
        let rt_descriptor_sets = (0..frames_in_flight)
            .map(|idx| {
                RhiDescriptorSet::<RtBindings>::new(
                    rhi,
                    rhi.descriptor_pool(),
                    &rt_descriptor_set_layout,
                    format!("RtDescriptorSet-{idx}"),
                )
            })
            .collect();

        Self {
            rt_descriptor_set_layout,
            rt_descriptor_sets,
            device: rhi.device.clone(),
        }
    }

    pub fn update(&self, frame_label: usize, tlas: &RhiAcceleration, output_image_view: &RhiImage2DView) {
        let crt_set = &self.rt_descriptor_sets[frame_label];

        let tlas_binding_item = RtBindings::tlas();
        let tlas_handle = tlas.handle();
        let mut acc_write_info = vk::WriteDescriptorSetAccelerationStructureKHR::default()
            .acceleration_structures(std::slice::from_ref(&tlas_handle));
        let write_tlas = vk::WriteDescriptorSet::default()
            .dst_set(crt_set.handle())
            .dst_binding(tlas_binding_item.binding)
            .dst_array_element(0)
            .descriptor_count(1)
            .descriptor_type(tlas_binding_item.descriptor_type)
            .push_next(&mut acc_write_info);

        let write_output_image = RtBindings::output_image().write_image(
            crt_set.handle(),
            0,
            vec![vk::DescriptorImageInfo {
                sampler: vk::Sampler::null(),
                image_view: output_image_view.handle(),
                image_layout: vk::ImageLayout::GENERAL,
            }],
        );
        let write_output_image = write_output_image.to_vk_type();

        unsafe {
            self.device.update_descriptor_sets(&[write_tlas, write_output_image], &[]);
        }
    }

    #[inline]
    pub fn rt_descriptor_set_layout(&self) -> &RhiDescriptorSetLayout<RtBindings> {
        &self.rt_descriptor_set_layout
    }

    #[inline]
    pub fn rt_descriptor_sets(&self) -> &[RhiDescriptorSet<RtBindings>] {
        &self.rt_descriptor_sets
    }
}
impl Drop for AccManager {
    fn drop(&mut self) {
        log::info!("Dropping AccManager");
        // RhiXXX is RAII
    }
}

/// 光追专用的 descriptor set bindings
#[derive(ShaderLayout)]
pub struct RtBindings {
    #[binding = 0]
    #[descriptor_type = "ACCELERATION_STRUCTURE_KHR"]
    #[stage = "RAYGEN_KHR | CLOSEST_HIT_KHR"]
    _tlas: (),

    #[binding = 1]
    #[descriptor_type = "STORAGE_IMAGE"]
    #[stage = "RAYGEN_KHR | CLOSEST_HIT_KHR"]
    _output_image: (),
}
