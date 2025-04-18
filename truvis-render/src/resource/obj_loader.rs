use crate::resource::mesh::MeshBuilder;

pub struct ObjLoader {}

impl ObjLoader {
    pub fn load(obj_file: impl AsRef<str>) {
        let (mut models, materials) = tobj::load_obj(
            obj_file.as_ref(),
            &tobj::LoadOptions {
                single_index: true,
                triangulate: true,
                ignore_lines: true,
                ignore_points: true,
                ..Default::default()
            },
        )
        .expect("Failed to OBJ load file");

        // Note: If you don't mind missing the materials, you can generate a default.
        let materials = materials.expect("Failed to load MTL file");

        println!("Number of models          = {}", models.len());
        println!("Number of materials       = {}", materials.len());

        let mut mesh_builder = MeshBuilder::default();
        for (i, m) in models.into_iter().enumerate() {
            // 等价于 memcpy
            mesh_builder.positions = m.mesh.positions;
            mesh_builder.normal = m.mesh.normals;
            mesh_builder.uv = m.mesh.texcoords;
            mesh_builder.index.push(m.mesh.indices);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::resource::obj_loader::ObjLoader;

    #[test]
    fn load_obj() {
        ObjLoader::load("assets/obj/spot.obj")
    }
}
