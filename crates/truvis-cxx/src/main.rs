use model_manager::component::{DrsInstance, DrsMaterial, DrsMesh};
use model_manager::guid_new_type::{InsGuid, MatGuid, MeshGuid};
use std::collections::HashMap;
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

fn main() {
    let rhi = Rhi::new("test".to_string(), vec![]);

    let mut mesh_map: HashMap<MeshGuid, DrsMesh> = HashMap::new();
    let mut mat_map: HashMap<MatGuid, DrsMaterial> = HashMap::new();
    let mut ins_map: HashMap<InsGuid, DrsInstance> = HashMap::new();

    let uuids = AssimpSceneLoader::load_scene(
        &rhi,
        std::path::Path::new("assets/obj/spot.obj"),
        |ins: DrsInstance| {
            let uuid = InsGuid::new();
            ins_map.insert(uuid, ins);
            uuid
        },
        |mesh: DrsMesh| {
            let uuid = MeshGuid::new();
            mesh_map.insert(uuid, mesh);
            uuid
        },
        |mat: DrsMaterial| {
            let uuid = MatGuid::new();
            mat_map.insert(uuid, mat);
            uuid
        },
    );

    println!("uuids: {:?}", uuids);
}
