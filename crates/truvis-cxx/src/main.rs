use std::collections::HashMap;
use model_manager::component::{TruInstance, TruMesh, TruMaterial};
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

fn main() {
    let rhi = Rhi::new("test".to_string(), vec![]);

    let mut mesh_map: HashMap<uuid::Uuid, TruMesh> = HashMap::new();
    let mut mat_map: HashMap<uuid::Uuid, TruMaterial> = HashMap::new();
    let mut ins_map: HashMap<uuid::Uuid, TruInstance> = HashMap::new();

    let uuids = AssimpSceneLoader::load_scene(
        &rhi,
        std::path::Path::new("assets/obj/spot.obj"),
        |ins: TruInstance| {
            let uuid = uuid::Uuid::new_v4();
            ins_map.insert(uuid, ins);
            uuid
        },
        |mesh: TruMesh| {
            let uuid = uuid::Uuid::new_v4();
            mesh_map.insert(uuid, mesh);
            uuid
        },
        |mat: TruMaterial| {
            let uuid = uuid::Uuid::new_v4();
            mat_map.insert(uuid, mat);
            uuid
        },
    );

    println!("uuids: {:?}", uuids);
}
