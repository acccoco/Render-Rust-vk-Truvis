use model_manager::manager::instance_manager::InstanceManager;
use model_manager::manager::mat_manager::MatManager;
use model_manager::manager::mesh_manager::MeshManager;
use truvis_assimp::SceneLoader;
use truvis_rhi::rhi::Rhi;

fn main() {
    let rhi = Rhi::new("test".to_string(), vec![]);

    let mut mesh_manager = MeshManager::default();
    let mut mat_manager = MatManager::default();
    let mut instance_manager = InstanceManager::default();

    let uuids = SceneLoader::load_model(
        &rhi,
        std::path::Path::new("assets/obj/spot.obj"),
        &mut instance_manager,
        &mut mesh_manager,
        &mut mat_manager,
    );

    println!("uuids: {:?}", uuids);
}
