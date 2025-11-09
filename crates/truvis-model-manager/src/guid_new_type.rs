/// 生成基于 uuid::Uuid 的新类型的宏
macro_rules! uuid_new_type {
    // 支持可见性修饰符的版本
    ($vis:vis $name:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
        $vis struct $name(pub uuid::Uuid);

        impl $name {
            /// 创建一个新的 UUID
            #[inline]
            pub fn new() -> Self {
                Self(uuid::Uuid::new_v4())
            }

            /// 从现有的 UUID 创建
            #[inline]
            pub fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }

            /// 获取内部的 UUID
            #[inline]
            pub fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }

            /// 转换为内部的 UUID
            #[inline]
            pub fn into_uuid(self) -> uuid::Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<uuid::Uuid> for $name {
            fn from(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }
        }

        impl From<$name> for uuid::Uuid {
            fn from(guid: $name) -> Self {
                guid.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                uuid::Uuid::from_str(s).map(Self)
            }
        }
    };

    // 不带可见性修饰符的版本（默认为私有）
    ($name:ident) => {
        uuid_new_type!( $name);
    };
}

// 使用宏重新定义现有的类型
uuid_new_type!(pub MeshGuid);
uuid_new_type!(pub MatGuid);
uuid_new_type!(pub InsGuid);
uuid_new_type!(pub LightGuid);
uuid_new_type!(pub TexGuid);
