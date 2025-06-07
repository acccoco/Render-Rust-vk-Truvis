/// 创建一个命名的数组和对应的枚举类型
#[macro_export]
macro_rules! create_named_array {
    ($enum_name:ident, $array_name:ident, $type:ty, [$(($variant:ident, $value:expr)),* $(,)?]) => {
        // 定义枚举
        #[repr(usize)]
        #[derive(Debug, Clone, Copy)]
        enum $enum_name {
            $($variant,)*
        }

        // 定义索引方法
        impl $enum_name {
            const COUNT: usize = count_indexed_array!($($variant),*);

            // 定义数组
            const ARRAY: [$type; Self::COUNT] = [
                $($value,)*
            ];

            pub fn value(self) -> &'static $type {
                &Self::ARRAY[self as usize]
            }

            pub const fn index(self) -> usize {
                self as usize
            }

            pub fn iter() -> impl Iterator<Item = Self> {
                (0..Self::COUNT).map(|i| unsafe { std::mem::transmute(i) })
            }
        }
    };
}

/// 辅助宏，计算变体数量
#[macro_export]
macro_rules! count_indexed_array {
    () => (0);
    ($head:tt $(, $tail:tt)*) => (1 + count_indexed_array!($($tail),*));
}
