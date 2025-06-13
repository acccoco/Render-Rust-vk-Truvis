mod triangle_pass;
mod triangle_pipeline;

use crate::triangle_pipeline::TrianglePipeline;
use imgui::{StyleColor, TextureId, Ui};
use model_manager::component::DrsGeometry;
use model_manager::vertex::vertex_pc::{VertexAosLayoutPosColor, VertexPosColor};
use truvis_render::app::{OuterApp, TruvisApp};
use truvis_render::platform::camera::DrsCamera;
use truvis_render::render::Renderer;
use truvis_render::render_pipeline::pipeline_context::PipelineContext;

struct HelloTriangle {
    triangle_pipeline: TrianglePipeline,
    triangle: DrsGeometry<VertexPosColor>,
    frame_id: usize,
}
impl OuterApp for HelloTriangle {
    fn init(renderer: &mut Renderer, _camera: &mut DrsCamera) -> Self {
        log::info!("hello triangle init.");

        Self {
            triangle_pipeline: TrianglePipeline::new(
                &renderer.rhi,
                &renderer.renderer_settings().pipeline_settings,
                renderer.bindless_mgr.clone(),
            ),
            triangle: VertexAosLayoutPosColor::triangle(&renderer.rhi),
            frame_id: 0,
        }
    }

    fn draw_ui(&mut self, ui: &mut Ui) {
        static mut UI_VALUE: usize = 0;

        let mut main_window_size = [0.0, 0.0];
        let mut main_window_pos = [0.0, 0.0];

        unsafe {
            let viewport = imgui::sys::igGetMainViewport();
            let viewport_size = (*viewport).Size;
            let viewport_pos = (*viewport).Pos;
            let root_node_id = imgui::sys::igGetID_Str(c"MainDockSpace".as_ptr());

            let window_flags = imgui::WindowFlags::NO_MOVE
                | imgui::WindowFlags::NO_TITLE_BAR
                | imgui::WindowFlags::MENU_BAR
                | imgui::WindowFlags::NO_COLLAPSE
                | imgui::WindowFlags::NO_BRING_TO_FRONT_ON_FOCUS
                | imgui::WindowFlags::NO_NAV_FOCUS
                | imgui::WindowFlags::NO_DOCKING
                | imgui::WindowFlags::NO_BACKGROUND
                | imgui::WindowFlags::NO_RESIZE;

            ui.window("main dock space")
                .position([0.0, 0.0], imgui::Condition::Always)
                .size([viewport_size.x, viewport_size.y], imgui::Condition::Always)
                .flags(window_flags)
                .build(|| {
                    if imgui::sys::igDockBuilderGetNode(root_node_id).is_null() {
                        imgui::sys::igDockBuilderRemoveNode(root_node_id);
                        imgui::sys::igDockBuilderAddNode(root_node_id, imgui::sys::ImGuiDockNodeFlags_NoCloseButton);
                        imgui::sys::igDockBuilderSetNodeSize(root_node_id, (*imgui::sys::igGetMainViewport()).Size);
                        // imgui::sys::igDockBuilderSetNodePos(root_id, imgui::sys::ImVec2 { x: 0.0, y: 0.0 });
                        // let root_node = imgui::sys::igDockBuilderGetNode(root_id);
                        // (*root_node).LocalFlags |= imgui::sys::ImGuiDockNodeFlags_HiddenTabBar;

                        let mut dock_main_id = root_node_id;
                        let dock_right_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Right,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );
                        let dock_left_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Left,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );
                        let dock_down_id = imgui::sys::igDockBuilderSplitNode(
                            dock_main_id,
                            imgui::sys::ImGuiDir_Down,
                            0.2,
                            std::ptr::null_mut(),
                            std::ptr::from_mut(&mut dock_main_id),
                        );

                        log::info!("main node id: {}", dock_main_id);
                        imgui::sys::igDockBuilderDockWindow(c"left".as_ptr(), dock_left_id);
                        imgui::sys::igDockBuilderDockWindow(c"right".as_ptr(), dock_right_id);
                        imgui::sys::igDockBuilderDockWindow(c"down".as_ptr(), dock_down_id);
                        imgui::sys::igDockBuilderDockWindow(c"render".as_ptr(), dock_main_id);
                        imgui::sys::igDockBuilderFinish(root_node_id);
                    }

                    // let main_node = imgui::sys::igDockBuilderGetCentralNode(root_node_id);
                    // log::info!("root node id: {}", root_node_id);
                    // let main_node_id = (*main_node).ID;
                    // log::info!("main node id: {}", main_node_id);
                    // imgui::sys::igDockBuilderDockWindow(c"render".as_ptr(), main_node_id);

                    imgui::sys::igDockSpace(
                        root_node_id,
                        imgui::sys::ImVec2 { x: 0.0, y: 0.0 },
                        imgui::sys::ImGuiDockNodeFlags_None as _,
                        std::ptr::null(),
                    );

                    main_window_size = ui.window_size();
                    main_window_pos = ui.window_pos();
                });

            ui.window("left")
                .size([100.0, 100.0], imgui::Condition::Always)
                // .movable(false)
                // .resizable(false)
                .draw_background(false)
                .title_bar(false)
                .menu_bar(false)
                .build(|| {
                    ui.text_wrapped("Hello world!");
                    ui.text_wrapped("こんにちは世界！");
                    ui.text_wrapped(format!("Frame ID: {}", self.frame_id));
                    let choices = ["test test this is 1", "test test this is 2"];
                    unsafe {
                        if ui.button(choices[UI_VALUE]) {
                            UI_VALUE += 1;
                            UI_VALUE %= 2;
                        }
                    }
                    ui.button("This...is...imgui-rs!");
                    ui.separator();
                    let mouse_pos = ui.io().mouse_pos;
                    ui.text(format!("Mouse Position: ({:.1},{:.1})", mouse_pos[0], mouse_pos[1]));
                });
            ui.window("render")
                .size([400.0, 400.0], imgui::Condition::Always)
                .title_bar(false)
                .resizable(false)
                // .bg_alpha(0.0)
                .draw_background(false)
                .build(|| {
                    ui.text("render window");
                    imgui::Image::new(TextureId::new(114), [400.0, 400.0]).build(ui);

                    let window_size = ui.window_size();
                    let window_pos = ui.window_pos();
                    ui.text(format!("Window Size: ({:.1},{:.1})", window_size[0], window_size[1]));
                    ui.text(format!("Window Position: ({:.1},{:.1})", window_pos[0], window_pos[1]));
                });
            ui.window("right").size([400.0, 400.0], imgui::Condition::Always).draw_background(false).build(|| {
                ui.text("test window.");
                let root_node = imgui::sys::igDockBuilderGetNode(root_node_id);
                let root_pos = (*root_node).Pos;
                let root_size = (*root_node).Size;
                ui.text(format!("Root Node Position: ({:.1},{:.1})", root_pos.x, root_pos.y));
                ui.text(format!("Root Node Size: ({:.1},{:.1})", root_size.x, root_size.y));

                ui.text(format!("Main Window Size: ({:.1},{:.1})", main_window_size[0], main_window_size[1]));
                ui.text(format!("Main Window Position: ({:.1},{:.1})", main_window_pos[0], main_window_pos[1]));

                // let center_node = imgui::sys::igDockBuilderGetCentralNode(root_id);
                // let center_pos = (*center_node).Pos;
                // let center_size = (*center_node).Size;
                // ui.text(format!("Center Node Position: ({:.1},{:.1})", center_pos.x, center_pos.y));
                // ui.text(format!("Center Node Size: ({:.1},{:.1})", center_size.x, center_size.y));
            });
            ui.window("down").build(|| {
                ui.text("down window");
                ui.text("This is a test window.");
                ui.text("You can put anything you want here.");
            });
        }
    }

    fn draw(&self, pipeline_ctx: PipelineContext) {
        self.triangle_pipeline.render(pipeline_ctx, &self.triangle);
    }
}

fn main() {
    TruvisApp::<HelloTriangle>::run();
}
