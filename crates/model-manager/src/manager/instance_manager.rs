use crate::component::instance::SimpleInstance;

// TODO 是否需要 Box 呢
#[derive(Default)]
pub struct InstanceManager {
    pub ins_map: std::collections::HashMap<uuid::Uuid, Box<SimpleInstance>>,
}

impl InstanceManager {
    pub fn register_instance(&mut self, instance: SimpleInstance) -> uuid::Uuid {
        let guid = uuid::Uuid::new_v4();
        self.ins_map.insert(guid, Box::new(instance));
        guid
    }
}
