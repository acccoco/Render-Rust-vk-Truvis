use model_manager::component::instance::SimpleInstance;
use model_manager::component::mat::SimpleMaterial;
use model_manager::component::mesh::SimpleMesh;
use std::collections::HashMap;
use truvis_cxx::AssimpSceneLoader;
use truvis_rhi::rhi::Rhi;

fn main() {
    let rhi = Rhi::new("test".to_string(), vec![]);

    let mut mesh_map: HashMap<uuid::Uuid, SimpleMesh> = HashMap::new();
    let mut mat_map: HashMap<uuid::Uuid, SimpleMaterial> = HashMap::new();
    let mut ins_map: HashMap<uuid::Uuid, SimpleInstance> = HashMap::new();

    let uuids = AssimpSceneLoader::load_model(
        &rhi,
        std::path::Path::new("assets/obj/spot.obj"),
        &mut (|ins: SimpleInstance| {
            let uuid = uuid::Uuid::new_v4();
            ins_map.insert(uuid, ins);
            uuid
        }),
        &mut (|mesh: SimpleMesh| {
            let uuid = uuid::Uuid::new_v4();
            mesh_map.insert(uuid, mesh);
            uuid
        }),
        &mut (|mat: SimpleMaterial| {
            let uuid = uuid::Uuid::new_v4();
            mat_map.insert(uuid, mat);
            uuid
        }),
    );

    println!("uuids: {:?}", uuids);
}
