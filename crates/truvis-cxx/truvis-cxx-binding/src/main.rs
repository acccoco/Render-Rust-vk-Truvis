use std::collections::HashMap;

use truvis_cxx_binding::AssimpSceneLoader;
use truvis_gfx::gfx::Gfx;
use truvis_model_manager::components::instance::Instance;
use truvis_model_manager::components::material::Material;
use truvis_model_manager::components::mesh::Mesh;
use truvis_model_manager::guid_new_type::{InsGuid, MatGuid, MeshGuid};

fn main() {
    Gfx::init("test".to_string(), vec![]);

    let mut mesh_map: HashMap<MeshGuid, Mesh> = HashMap::new();
    let mut mat_map: HashMap<MatGuid, Material> = HashMap::new();
    let mut ins_map: HashMap<InsGuid, Instance> = HashMap::new();

    let uuids = AssimpSceneLoader::load_scene(
        std::path::Path::new("assets/obj/spot.obj"),
        |ins: Instance| {
            let uuid = InsGuid::new();
            ins_map.insert(uuid, ins);
            uuid
        },
        |mesh: Mesh| {
            let uuid = MeshGuid::new();
            mesh_map.insert(uuid, mesh);
            uuid
        },
        |mat: Material| {
            let uuid = MatGuid::new();
            mat_map.insert(uuid, mat);
            uuid
        },
    );

    println!("uuids: {:?}", uuids);
}
