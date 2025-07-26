use std::collections::HashMap;
use model_manager::component::{DrsInstance, DrsMesh, DrsMaterial};
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

fn main() {
    let rhi = Rhi::new("test".to_string(), vec![]);

    let mut mesh_map: HashMap<uuid::Uuid, DrsMesh> = HashMap::new();
    let mut mat_map: HashMap<uuid::Uuid, DrsMaterial> = HashMap::new();
    let mut ins_map: HashMap<uuid::Uuid, DrsInstance> = HashMap::new();

    let uuids = AssimpSceneLoader::load_scene(
        &rhi,
        std::path::Path::new("assets/obj/spot.obj"),
        |ins: DrsInstance| {
            let uuid = uuid::Uuid::new_v4();
            ins_map.insert(uuid, ins);
            uuid
        },
        |mesh: DrsMesh| {
            let uuid = uuid::Uuid::new_v4();
            mesh_map.insert(uuid, mesh);
            uuid
        },
        |mat: DrsMaterial| {
            let uuid = uuid::Uuid::new_v4();
            mat_map.insert(uuid, mat);
            uuid
        },
    );

    println!("uuids: {:?}", uuids);
}
